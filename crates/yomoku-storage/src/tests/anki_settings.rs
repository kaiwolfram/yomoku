use rusqlite::types::Value;
use yomoku_core::{
    anki::{AnkiDeckId, AnkiModelCreation, AnkiModelLayout},
    document::{PasteText, TextOffset},
    pagination::{PageLength, ReadingPosition},
    typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography},
};

use super::*;
use crate::{anki::AnkiSubmissionOutcome, anki_model::AnkiModelCreationOutcome};

// Preserve all existing table data, including history, allocation high-water
// marks and exact journal payloads. Only settings and migration history differ.
fn other_state(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let names = connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table'
             AND name NOT IN ('schema_migrations', 'anki_settings') ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    names
        .into_iter()
        .map(|name| {
            let mut statement = connection
                .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
                .unwrap();
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();
            (name, rows)
        })
        .collect()
}

#[test]
fn anki_settings_start_unconfigured_and_exact_ids_and_clear_survive_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.anki_default_deck().unwrap(), None);
    for value in [Some(1), Some(9007199254740993), Some(i64::MAX), None] {
        let deck = value.map(|id| AnkiDeckId::try_from(id).unwrap());
        assert_eq!(storage.save_anki_default_deck(deck).unwrap(), deck);
        drop(storage);
        let before = fs::read(database.path()).unwrap();
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.anki_default_deck().unwrap(), deck);
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    for invalid in [0, -1, i64::MIN] {
        assert!(AnkiDeckId::try_from(invalid).is_err());
        assert!(storage
            .connection
            .execute("UPDATE anki_settings SET default_deck_id = ?1", [invalid])
            .is_err());
    }
    assert!(storage
        .connection
        .execute("INSERT INTO anki_settings VALUES (2, 1)", [])
        .is_err());
    assert!(storage
        .connection
        .execute("UPDATE anki_settings SET default_deck_id = 1.5", [])
        .is_err());
    assert_eq!(storage.anki_default_deck().unwrap(), None);
}

#[test]
fn anki_settings_upgrade_and_changes_preserve_reader_history_and_both_journal_kinds() {
    for model in [false, true] {
        for uncertain in [false, true] {
            let database = TestDatabase::new();
            let mut connection = Connection::open(database.path()).unwrap();
            connection
                .pragma_update(None, "foreign_keys", true)
                .unwrap();
            migrate(&mut connection, &MIGRATIONS[..23]).unwrap();
            super::history_schema::seed_history(&connection, 23);
            let mut storage = Storage { connection };
            let document = storage
                .create_paste(&PasteText::new("𠮷本文。".repeat(300)).unwrap())
                .unwrap();
            storage
                .save_reading_position(
                    document.document_id,
                    ReadingPosition {
                        source_id: document.chapter.source_id,
                        chapter_id: document.chapter.chapter_id,
                        offset: TextOffset::at(&document.chapter.text, 4).unwrap(),
                    },
                )
                .unwrap();
            storage.save_page_length(PageLength::Characters750).unwrap();
            storage
                .save_reader_typography(ReaderTypography {
                    font_size: ReaderFontSize::Sp24,
                    line_spacing: ReaderLineSpacing::Percent200,
                    furigana: true,
                })
                .unwrap();
            if model {
                let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
                let live = storage.begin_anki_model_creation(&creation).unwrap();
                if uncertain {
                    storage
                        .finish_anki_model_creation(
                            &live,
                            AnkiModelCreationOutcome::Uncertain {
                                known_model_id: Some(i64::MAX.try_into().unwrap()),
                            },
                        )
                        .unwrap();
                }
            } else {
                let live = storage
                    .begin_anki_submission(&super::anki::payload(9007199254740993))
                    .unwrap();
                if uncertain {
                    storage
                        .finish_anki_submission(
                            &live,
                            AnkiSubmissionOutcome::Uncertain {
                                known_note_id: Some(i64::MAX.try_into().unwrap()),
                            },
                        )
                        .unwrap();
                }
            }
            let before = other_state(&storage.connection);
            let recovery = storage.anki_recovery_snapshot().unwrap();
            drop(storage);

            let mut storage = Storage::open(database.path()).unwrap();
            assert_eq!(storage.schema_version().unwrap(), 24);
            assert_eq!(storage.anki_default_deck().unwrap(), None);
            assert_eq!(other_state(&storage.connection), before);
            for deck in [Some(1), Some(i64::MAX), None] {
                storage
                    .save_anki_default_deck(deck.map(|id| id.try_into().unwrap()))
                    .unwrap();
                assert_eq!(other_state(&storage.connection), before);
                assert_eq!(storage.anki_recovery_snapshot().unwrap(), recovery);
                // Preferences never release either durable mutation gate.
                assert!(storage
                    .begin_anki_submission(&super::anki::payload(1))
                    .is_err());
                assert!(storage
                    .begin_anki_model_creation(
                        &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap()
                    )
                    .is_err());
                drop(storage);
                storage = Storage::open(database.path()).unwrap();
                assert_eq!(other_state(&storage.connection), before);
                assert_eq!(storage.anki_recovery_snapshot().unwrap(), recovery);
            }
        }
    }
}

#[test]
fn anki_settings_statement_and_commit_failures_keep_previous_value_and_allow_retry() {
    for trigger in [
        "CREATE TRIGGER fail_deck BEFORE UPDATE ON anki_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_deck (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_deck AFTER UPDATE ON anki_settings BEGIN INSERT INTO deferred_deck VALUES (-1); END;",
    ] {
        for requested in [None, Some(i64::MAX)] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let previous = Some(AnkiDeckId::try_from(9007199254740993).unwrap());
            storage.save_anki_default_deck(previous).unwrap();
            storage.connection.execute_batch(trigger).unwrap();
            let before = fs::read(database.path()).unwrap();
            let requested = requested.map(|id| id.try_into().unwrap());
            assert!(matches!(
                storage.save_anki_default_deck(requested),
                Err(StorageError::Sqlite(_))
            ));
            assert!(storage.connection.is_autocommit());
            assert_eq!(fs::read(database.path()).unwrap(), before);
            assert_eq!(storage.anki_default_deck().unwrap(), previous);
            assert_eq!(Storage::open(database.path()).unwrap().anki_default_deck().unwrap(), previous);
            storage.connection.execute_batch("DROP TRIGGER fail_deck").unwrap();
            assert_eq!(storage.save_anki_default_deck(requested).unwrap(), requested);
        }
    }
}

#[test]
fn anki_settings_corrupt_or_missing_row_is_an_error_without_repair_or_writes() {
    for sql in [
        "DELETE FROM anki_settings",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_settings SET default_deck_id = 0",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_settings SET default_deck_id = -1",
        "PRAGMA ignore_check_constraints = ON; UPDATE anki_settings SET id = 2",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        let before = fs::read(database.path()).unwrap();
        for requested in [None, Some(i64::MAX.try_into().unwrap())] {
            assert!(matches!(
                storage.anki_default_deck(),
                Err(StorageError::InvalidSettings)
            ));
            assert!(matches!(
                storage.save_anki_default_deck(requested),
                Err(StorageError::InvalidSettings)
            ));
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
        drop(storage);
        let storage = Storage::open(database.path()).unwrap();
        assert!(matches!(
            storage.anki_default_deck(),
            Err(StorageError::InvalidSettings)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn anki_settings_ignored_or_rewritten_update_rolls_back_without_acknowledgement() {
    for trigger in [
        "CREATE TRIGGER change_deck BEFORE UPDATE ON anki_settings BEGIN SELECT RAISE(IGNORE); END;",
        "CREATE TRIGGER change_deck AFTER UPDATE ON anki_settings BEGIN UPDATE anki_settings SET default_deck_id = 2; END;",
        "CREATE TRIGGER change_deck AFTER UPDATE ON anki_settings BEGIN DELETE FROM anki_settings; END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(storage.save_anki_default_deck(Some(1.try_into().unwrap())), Err(StorageError::InvalidSettings)));
        assert!(storage.connection.is_autocommit());
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert_eq!(storage.anki_default_deck().unwrap(), None);
        storage.connection.execute_batch("DROP TRIGGER change_deck").unwrap();
        assert!(storage.save_anki_default_deck(Some(1.try_into().unwrap())).is_ok());
    }
}

#[test]
fn anki_settings_failed_migration_rolls_back_table_and_default_before_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..23]).unwrap();
    super::history_schema::seed_history(&connection, 23);
    connection.execute_batch("CREATE TRIGGER fail_deck_migration BEFORE INSERT ON schema_migrations WHEN NEW.version = 24 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let before = other_state(&connection);
    drop(connection);
    let bytes = fs::read(database.path()).unwrap();
    let before_history = history(&database.path());
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert_eq!(history(&database.path()), before_history);
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection.table_exists(None, "anki_settings").unwrap());
    assert_eq!(other_state(&connection), before);
    connection
        .execute_batch("DROP TRIGGER fail_deck_migration")
        .unwrap();
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 24);
    assert_eq!(storage.anki_default_deck().unwrap(), None);
    assert_eq!(other_state(&storage.connection), before);
}

#[test]
fn anki_settings_concurrent_writers_keep_one_complete_preference_without_touching_journals() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .begin_anki_submission(&super::anki::payload(1))
        .unwrap();
    let before = other_state(&storage.connection);
    let ready = Arc::new(Barrier::new(3));
    std::thread::scope(|scope| {
        for value in [None, Some(9007199254740993), Some(i64::MAX)] {
            let path = database.path();
            let ready = ready.clone();
            scope.spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                ready.wait();
                let deck = value.map(|id| id.try_into().unwrap());
                for _ in 0..10 {
                    assert_eq!(storage.save_anki_default_deck(deck).unwrap(), deck);
                }
            });
        }
    });
    let final_deck = storage.anki_default_deck().unwrap().map(i64::from);
    assert!([None, Some(9007199254740993), Some(i64::MAX)].contains(&final_deck));
    assert_eq!(other_state(&storage.connection), before);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .anki_default_deck()
            .unwrap()
            .map(i64::from),
        final_deck
    );
}
