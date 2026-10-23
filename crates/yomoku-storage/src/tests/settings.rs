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
