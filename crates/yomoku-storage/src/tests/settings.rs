use yomoku_core::{document::PasteText, pagination::PageLength};

use super::*;

#[test]
fn every_preset_is_global_durable_and_does_not_change_documents_or_positions() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    let position = yomoku_core::pagination::ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: yomoku_core::document::TextOffset::at(&document.chapter.text, 4).unwrap(),
    };
    let expected = storage
        .save_document_reading_position(document.document_id, position)
        .map(|value| value.map(crate::tests::as_paste))
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
fn all_typography_preferences_persist_without_changing_logical_pages_or_document_rows() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&PasteText::new("𠮷本文。\n".repeat(300)).unwrap())
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
