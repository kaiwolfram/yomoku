use rusqlite::types::Value;
use yomoku_core::document::DocumentId;

use super::*;

// Insert the shapes actually available at each historical version. Current
// repository readers deliberately require the current schema.
fn seed_history(connection: &Connection, version: usize) {
    connection
        .execute_batch(
            "INSERT INTO sources VALUES (41, 'paste', '原本 猫 𠮷 が');
             INSERT INTO chapters VALUES (51, 41, 0);
             INSERT INTO documents (id, source_id, title) VALUES (61, 41, 'renamed');
             INSERT INTO chapter_texts VALUES (51, '猫 猫');
             INSERT INTO completion_commands VALUES (71);
             INSERT INTO completion_commands VALUES (72);
             INSERT INTO reading_completions
                 (command_id, semantics_version, document_id, source_id, chapter_id,
                  start_byte_offset, end_byte_offset, departing_byte_offset,
                  page_length, completed_at_unix_ms)
                 VALUES (71, 1, 61, 41, 51, 0, 7, 0, 500, 300);",
        )
        .unwrap();
    if version >= 6 {
        connection
            .execute_batch(
                "INSERT INTO vocabulary VALUES (81, '猫', 'new', 100);
                 INSERT INTO vocabulary_status_events
                     (id, semantics_version, vocabulary_id, old_status, new_status,
                      cause, occurred_at_unix_ms)
                     VALUES (91, 1, 81, NULL, 'new', 'registration', 100),
                            (92, 1, 81, 'new', 'learning', 'manual', 200),
                            (93, 1, 81, 'learning', 'new', 'manual', 250);",
            )
            .unwrap();
    }
    if version >= 9 {
        connection
            .execute("UPDATE reading_completions SET active_time_ms = 1234", [])
            .unwrap();
    }
    if version >= 10 {
        // Historical identity intentionally needs no live chapter_analyses row.
        connection
            .execute_batch(
                "INSERT INTO completion_analyses VALUES
                     (71, 1, 101, 'engine/adapter-v1', 'dictionary/hash', 'b');
                 INSERT INTO reading_occurrences VALUES
                     (71, 0, 81, 0, 3, '猫', 'ネコ', '猫'),
                     (71, 1, 81, 4, 7, '猫', NULL, NULL);
                 UPDATE vocabulary SET status = 'known';
                 INSERT INTO vocabulary_status_events VALUES
                     (94, 1, 81, 'new', 'known', 'completion', 300, 71);",
            )
            .unwrap();
    }
    if (20..23).contains(&version) {
        connection
            .execute(
                "INSERT INTO vocabulary_dictionary_forms VALUES (81, '猫')",
                [],
            )
            .unwrap();
    }
    // Keep allocation high-water marks beyond all remaining rows through rebuilds.
    connection
        .execute(
            "UPDATE sqlite_sequence SET seq = 500 WHERE name IN
                ('sources', 'chapters', 'documents', 'completion_commands',
                 'vocabulary', 'vocabulary_status_events')",
            [],
        )
        .unwrap();
}

fn rows(connection: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut statement = connection.prepare(sql).unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn history_snapshot(connection: &Connection, version: usize) -> Vec<Vec<Vec<Value>>> {
    let mut snapshot = [
        "SELECT * FROM sources ORDER BY id",
        "SELECT * FROM chapters ORDER BY id",
        "SELECT * FROM completion_commands ORDER BY id",
    ]
    .map(|sql| rows(connection, sql))
    .to_vec();
    let time = if version >= 9 { "active_time_ms" } else { "0" };
    snapshot.push(rows(
        connection,
        &format!(
            "SELECT command_id, semantics_version, document_id, source_id, chapter_id,
                start_byte_offset, end_byte_offset, departing_byte_offset,
                page_length, completed_at_unix_ms, {time}
         FROM reading_completions ORDER BY command_id"
        ),
    ));
    snapshot.push(if version >= 6 {
        rows(connection, "SELECT * FROM vocabulary ORDER BY id")
    } else {
        vec![]
    });
    snapshot.push(if version >= 6 {
        let command = if version >= 10 {
            "completion_command_id"
        } else {
            "NULL"
        };
        rows(
            connection,
            &format!(
                "SELECT id, semantics_version, vocabulary_id, old_status, new_status,
                    cause, occurred_at_unix_ms, {command}
             FROM vocabulary_status_events ORDER BY id"
            ),
        )
    } else {
        vec![]
    });
    for sql in [
        "SELECT * FROM completion_analyses ORDER BY command_id",
        "SELECT * FROM reading_occurrences ORDER BY command_id, token_index",
    ] {
        snapshot.push(if version >= 10 {
            rows(connection, sql)
        } else {
            vec![]
        });
    }
    snapshot
}

fn assert_integrity(connection: &Connection) {
    assert_eq!(
        rows(connection, "PRAGMA integrity_check"),
        vec![vec![Value::Text("ok".into())]]
    );
    assert!(rows(connection, "PRAGMA foreign_key_check").is_empty());
}

fn expected_status_timeline(version: usize) -> Vec<yomoku_core::statistics::StatusTimelinePoint> {
    use yomoku_core::{library::WordStatusCounts, statistics::StatusTimelinePoint};
    let mut points = Vec::new();
    if version >= 6 {
        for (date, new, learning, known) in [
            (100, 1, 0, 0),
            (200, 0, 1, 0),
            (250, 1, 0, 0),
            (300, 0, 0, 1),
        ] {
            if date == 300 && version < 10 {
                continue;
            }
            points.push(StatusTimelinePoint {
                occurred_at_unix_ms: date,

                counts: WordStatusCounts {
                    new,
                    learning,
                    known,
                    ..WordStatusCounts::default()
                },
            });
        }
    }
    points
}

#[test]
fn every_history_schema_upgrades_without_reinterpreting_events_or_deleted_source_metadata() {
    for version in 5..=MIGRATIONS.len() {
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
        seed_history(&connection, version);
        let before = history_snapshot(&connection, version);
        drop(connection);

        let mut storage = Storage::open(database.path()).unwrap();
        let expected_totals = yomoku_core::statistics::LifetimeReadingTotals {
            japanese_occurrences: if version >= 10 { 2 } else { 0 },
            active_time_ms: if version >= 10 { 1234 } else { 0 },
        };
        assert_eq!(storage.lifetime_reading_totals().unwrap(), expected_totals);
        assert_eq!(
            storage.vocabulary_status_timeline().unwrap(),
            expected_status_timeline(version),
            "v{version}"
        );
        assert_eq!(
            history_snapshot(&storage.connection, MIGRATIONS.len()),
            before,
            "v{version}"
        );
        // Legacy completions stay explicitly unanalyzed, never zero-word analyses.
        assert!(rows(
            &storage.connection,
            "SELECT * FROM effective_completion_commands"
        )
        .is_empty());
        storage
            .delete_document(DocumentId::try_from(61).unwrap())
            .unwrap();
        assert!(rows(&storage.connection, "SELECT * FROM chapter_texts").is_empty());
        assert!(rows(&storage.connection, "SELECT * FROM documents").is_empty());
        assert_eq!(
            history_snapshot(&storage.connection, MIGRATIONS.len()),
            before,
            "deleted v{version}"
        );
        // Historical source/chapter/date/anchors can be joined without live content.
        assert_eq!(
            rows(
                &storage.connection,
                "SELECT c.document_id, s.id, s.kind, s.original_title, h.id, h.chapter_index,
                    c.completed_at_unix_ms, c.start_byte_offset, c.end_byte_offset
             FROM reading_completions c JOIN sources s ON s.id = c.source_id
             JOIN chapters h ON h.id = c.chapter_id AND h.source_id = s.id"
            ),
            vec![vec![
                Value::Integer(61),
                Value::Integer(41),
                Value::Text("paste".into()),
                Value::Text("原本 猫 𠮷 が".into()),
                Value::Integer(51),
                Value::Integer(0),
                Value::Integer(300),
                Value::Integer(0),
                Value::Integer(7)
            ]]
        );
        assert_integrity(&storage.connection);
        assert_eq!(storage.lifetime_reading_totals().unwrap(), expected_totals);
        assert_eq!(
            storage.vocabulary_status_timeline().unwrap(),
            expected_status_timeline(version),
            "deleted v{version}"
        );
        drop(storage);

        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.lifetime_reading_totals().unwrap(), expected_totals);
        assert_eq!(
            storage.vocabulary_status_timeline().unwrap(),
            expected_status_timeline(version),
            "reopened v{version}"
        );
        assert_eq!(
            history_snapshot(&storage.connection, MIGRATIONS.len()),
            before,
            "reopened v{version}"
        );
        assert_eq!(i64::from(storage.new_completion_command_id().unwrap()), 501);
        let document = storage
            .create_paste(&yomoku_core::document::PasteText::new("新しい本".into()).unwrap())
            .unwrap();
        assert_eq!(i64::from(document.document_id), 501);
        assert_eq!(i64::from(document.chapter.source_id), 501);
        assert_eq!(i64::from(document.chapter.chapter_id), 501);
        if version >= 6 {
            storage
                .register_vocabulary(
                    &[yomoku_core::vocabulary::Surface::new("犬").unwrap()],
                    1000,
                )
                .unwrap();
            assert_eq!(
                rows(
                    &storage.connection,
                    "SELECT MAX(id) FROM vocabulary_status_events"
                ),
                vec![vec![Value::Integer(501)]]
            );
        }
        assert_integrity(&storage.connection);
    }
}

#[test]
fn failed_late_upgrade_preserves_populated_history_and_retries_without_partial_rebuilds() {
    for version in [9, 10, 13, 17, 18] {
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
        seed_history(&connection, version);
        connection
            .execute_batch("CREATE VIEW library_read_clock AS SELECT 1")
            .unwrap();
        let before = history_snapshot(&connection, version);
        drop(connection);
        let bytes = fs::read(database.path()).unwrap();
        assert!(matches!(
            Storage::open(database.path()),
            Err(StorageError::Sqlite(_))
        ));
        assert_eq!(fs::read(database.path()).unwrap(), bytes, "v{version}");
        assert_eq!(history(&database.path()).len(), version);
        let connection = Connection::open(database.path()).unwrap();
        assert_eq!(history_snapshot(&connection, version), before);
        assert_integrity(&connection);
        connection
            .execute_batch("DROP VIEW library_read_clock")
            .unwrap();
        drop(connection);
        let storage = Storage::open(database.path()).unwrap();
        assert_eq!(
            history_snapshot(&storage.connection, MIGRATIONS.len()),
            before
        );
        assert_integrity(&storage.connection);
    }
}

// Snapshot every remaining table so the scope reduction cannot silently discard
// reader state, vocabulary/status history, dictionaries or Anki obligations.
fn state_without_known_form_links(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let mut statement = connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table'
         AND name NOT IN ('schema_migrations', 'vocabulary_dictionary_forms') ORDER BY name",
        )
        .unwrap();
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    names
        .into_iter()
        .map(|name| {
            let data = rows(
                connection,
                &format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")),
            );
            (name, data)
        })
        .collect()
}

#[test]
fn removing_known_forms_migrates_populated_database_and_preserves_all_other_state() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..22]).unwrap();
    seed_history(&connection, 22);
    assert_eq!(
        rows(&connection, "SELECT * FROM vocabulary_dictionary_forms").len(),
        1
    );
    let before = state_without_known_form_links(&connection);
    drop(connection);
    for _ in 0..2 {
        let storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 23);
        assert!(!storage
            .connection
            .table_exists(None, "vocabulary_dictionary_forms")
            .unwrap());
        assert_eq!(state_without_known_form_links(&storage.connection), before);
        assert_eq!(
            storage.vocabulary_status_timeline().unwrap(),
            expected_status_timeline(22)
        );
        assert_eq!(
            storage
                .lifetime_reading_totals()
                .unwrap()
                .japanese_occurrences,
            2
        );
        assert_integrity(&storage.connection);
    }
}

#[test]
fn failed_known_forms_removal_restores_links_and_history_before_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..22]).unwrap();
    seed_history(&connection, 22);
    let before = fs::read(database.path()).unwrap();
    let mut failing = MIGRATIONS[..22]
        .iter()
        .map(|migration| Migration { ..*migration })
        .collect::<Vec<_>>();
    failing.push(Migration {
        version: 23,
        name: MIGRATIONS[22].name,
        sql: concat!(
            include_str!("../../migrations/0023_remove_known_forms.sql"),
            "INSERT INTO nonexistent VALUES (1);"
        ),
    });
    assert!(matches!(
        migrate(&mut connection, &failing),
        Err(StorageError::Sqlite(_))
    ));
    assert!(connection.is_autocommit());
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(
        rows(&connection, "SELECT * FROM vocabulary_dictionary_forms").len(),
        1
    );
    assert_integrity(&connection);
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert!(!storage
        .connection
        .table_exists(None, "vocabulary_dictionary_forms")
        .unwrap());
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        expected_status_timeline(22)
    );
}
