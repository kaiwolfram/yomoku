use yomoku_core::{document::PasteText, pagination::PageLength};

use super::*;

#[test]
fn settings_upgrade_defaults_to_500_preserving_text_and_exact_position() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..3]).unwrap();
    connection.execute_batch("INSERT INTO sources VALUES (1, 'paste', '𠮷'); INSERT INTO chapters VALUES (1, 1, 0); INSERT INTO documents VALUES (1, 1, '𠮷'); INSERT INTO chapter_texts VALUES (1, '𠮷本文'); INSERT INTO reading_positions VALUES (1, 1, 4);").unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters500);
    let document = storage
        .load_paste(yomoku_core::document::DocumentId::try_from(1).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(document.chapter.text, "𠮷本文");
    assert_eq!(i64::from(document.reading_position.unwrap().offset), 4);
}

#[test]
fn every_preset_is_global_durable_and_does_not_change_documents_or_positions() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    let position = yomoku_core::pagination::ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: yomoku_core::document::TextOffset::at(&document.chapter.text, 4).unwrap(),
    };
    let expected = storage
        .save_reading_position(document.document_id, position)
        .unwrap();
    for length in PageLength::ALL {
        assert_eq!(storage.save_page_length(length).unwrap(), length);
        assert_eq!(storage.page_length().unwrap(), length);
        let mut reopened = Storage::open(database.path()).unwrap();
        assert_eq!(reopened.page_length().unwrap(), length);
        assert_eq!(reopened.load_paste(document.document_id).unwrap(), expected);
    }
}

#[test]
fn update_and_commit_failures_preserve_setting_and_allow_retry() {
    for trigger in [
        "CREATE TRIGGER fail_settings BEFORE UPDATE ON reader_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_settings AFTER UPDATE ON reader_settings BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.save_page_length(PageLength::Characters750).unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.save_page_length(PageLength::Characters250), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
        assert_eq!(Storage::open(database.path()).unwrap().page_length().unwrap(), PageLength::Characters750);
        storage.connection.execute_batch("DROP TRIGGER fail_settings").unwrap();
        assert_eq!(storage.save_page_length(PageLength::Characters250).unwrap(), PageLength::Characters250);
    }
}

#[test]
fn invalid_or_missing_settings_are_rejected_without_repair() {
    for sql in [
        "DELETE FROM reader_settings",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET page_length = 501;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(
            storage.page_length(),
            Err(StorageError::InvalidSettings)
        ));
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.save_page_length(PageLength::Characters250),
            Err(StorageError::InvalidSettings)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
        let reopened = Storage::open(database.path()).unwrap();
        assert!(matches!(
            reopened.page_length(),
            Err(StorageError::InvalidSettings)
        ));
    }
}

#[test]
fn typography_migration_preserves_existing_page_length_text_position_and_analysis() {
    use yomoku_core::typography::ReaderTypography;
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..14]).unwrap();
    let mut old = Storage { connection };
    old.save_page_length(PageLength::Characters750).unwrap();
    let document = old
        .create_paste(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    let position = yomoku_core::pagination::ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: yomoku_core::document::TextOffset::at(&document.chapter.text, 4).unwrap(),
    };
    let expected = old
        .save_reading_position(document.document_id, position)
        .unwrap();
    drop(old);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.reader_typography().unwrap(),
        ReaderTypography::default()
    );
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    let snapshot = storage
        .load_document_reader_snapshot(document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.typography, ReaderTypography::default());
    assert_eq!(snapshot.document.reading_position, Some(position));
}

#[test]
fn all_typography_preferences_persist_without_changing_logical_pages_or_document_rows() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .create_paste(&PasteText::new("𠮷本文。\n".repeat(300)).unwrap())
        .unwrap();
    let before = storage
        .load_document_reader_snapshot(document.document_id)
        .unwrap()
        .unwrap();
    for furigana in [false, true] {
        for font_size in ReaderFontSize::ALL {
            for line_spacing in ReaderLineSpacing::ALL {
                let typography = ReaderTypography {
                    font_size,
                    line_spacing,
                    furigana,
                };
                assert_eq!(
                    storage.save_reader_typography(typography).unwrap(),
                    typography
                );
                let mut reopened = Storage::open(database.path()).unwrap();
                assert_eq!(reopened.reader_typography().unwrap(), typography);
                let after = reopened
                    .load_document_reader_snapshot(document.document_id)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    after,
                    yomoku_core::reader::DocumentReaderSnapshot {
                        last_read_sequence: None,
                        typography,
                        ..before.clone()
                    }
                );
                assert_eq!(storage.page_length().unwrap(), PageLength::default());
            }
        }
    }
}

#[test]
fn typography_statement_and_commit_failure_roll_back_all_fields_and_retry() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let changed = ReaderTypography {
        font_size: ReaderFontSize::Sp24,
        line_spacing: ReaderLineSpacing::Percent200,
        furigana: true,
    };
    for sql in [
        "CREATE TRIGGER fail_typography BEFORE UPDATE ON reader_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_typography (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_typography AFTER UPDATE ON reader_settings BEGIN INSERT INTO deferred_typography VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.save_reader_typography(changed), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.reader_typography().unwrap(), ReaderTypography::default());
        assert_eq!(Storage::open(database.path()).unwrap().reader_typography().unwrap(), ReaderTypography::default());
        storage.connection.execute_batch("DROP TRIGGER fail_typography").unwrap();
        assert_eq!(storage.save_reader_typography(changed).unwrap(), changed);
    }
}

#[test]
fn corrupt_typography_is_rejected_without_writes_or_silent_repair() {
    use yomoku_core::typography::ReaderTypography;
    for sql in [
        "DELETE FROM reader_settings",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET font_size = 17;",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET line_spacing = 151;",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET furigana = -1;",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET furigana = 2;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.reader_typography(),
            Err(StorageError::InvalidSettings)
        ));
        assert!(matches!(
            storage.save_reader_typography(ReaderTypography::default()),
            Err(StorageError::InvalidSettings)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn typography_migration_failure_rolls_back_first_column_and_history_before_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..14]).unwrap();
    connection.execute_batch("UPDATE reader_settings SET page_length = 750; ALTER TABLE reader_settings ADD COLUMN line_spacing INTEGER NOT NULL DEFAULT 150;").unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    let before_history = history(&database.path());
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(history(&database.path()), before_history);
    let connection = Connection::open(database.path()).unwrap();
    assert!(connection
        .prepare("SELECT font_size FROM reader_settings")
        .is_err());
    connection
        .execute_batch("ALTER TABLE reader_settings DROP COLUMN line_spacing")
        .unwrap();
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.reader_typography().unwrap(), Default::default());
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
}

#[test]
fn furigana_migration_defaults_off_preserving_existing_preferences_and_exact_position() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..15]).unwrap();
    let mut old = Storage { connection };
    old.connection
        .execute_batch(
            "UPDATE reader_settings SET font_size = 24, line_spacing = 200, page_length = 750",
        )
        .unwrap();
    let document = old
        .create_paste(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    let position = yomoku_core::pagination::ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: yomoku_core::document::TextOffset::at(&document.chapter.text, 4).unwrap(),
    };
    let expected = old
        .save_reading_position(document.document_id, position)
        .unwrap();
    drop(old);
    let mut storage = Storage::open(database.path()).unwrap();
    let typography = ReaderTypography {
        font_size: ReaderFontSize::Sp24,
        line_spacing: ReaderLineSpacing::Percent200,
        furigana: false,
    };
    assert_eq!(storage.reader_typography().unwrap(), typography);
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    let snapshot = storage
        .load_document_reader_snapshot(document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.typography, typography);
    assert_eq!(snapshot.document.reading_position, Some(position));
    let before = fs::read(database.path()).unwrap();
    drop(storage);
    let reopened = Storage::open(database.path()).unwrap();
    assert_eq!(reopened.reader_typography().unwrap(), typography);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn furigana_migration_failure_preserves_schema_and_history_before_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..15]).unwrap();
    connection.execute_batch("UPDATE reader_settings SET font_size = 24, line_spacing = 200; ALTER TABLE reader_settings ADD COLUMN furigana INTEGER NOT NULL DEFAULT 0").unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    let before_history = history(&database.path());
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(history(&database.path()), before_history);
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch("ALTER TABLE reader_settings DROP COLUMN furigana")
        .unwrap();
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    let typography = storage.reader_typography().unwrap();
    assert!(!typography.furigana);
    assert_eq!(typography.font_size.sp(), 24);
    assert_eq!(typography.line_spacing.percent(), 200);
}
