use yomoku_core::{document::PasteText, pagination::PageLength};

use super::*;

#[test]
fn fresh_reader_page_length_defaults_to_250_and_reopens_unchanged() {
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(PageLength::default(), PageLength::Characters250);
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters250);
    drop(storage);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .page_length()
            .unwrap(),
        PageLength::Characters250
    );
}

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
    for font_size in ReaderFontSize::ALL {
        for line_spacing in ReaderLineSpacing::ALL {
            let typography = ReaderTypography {
                font_size,
                line_spacing,
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

#[test]
fn typography_statement_and_commit_failure_roll_back_all_fields_and_retry() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let changed = ReaderTypography {
        font_size: ReaderFontSize::Sp24,
        line_spacing: ReaderLineSpacing::Percent200,
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
fn statistics_timeframe_defaults_to_last_week_and_reopens_with_other_settings() {
    use yomoku_core::statistics::StatisticsTimeframe::*;
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
    connection
        .execute("UPDATE reader_settings SET page_length = 750", [])
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 2);
    assert_eq!(storage.statistics_timeframe().unwrap(), LastWeek);
    assert_eq!(storage.page_length().unwrap().characters(), 750);
    for value in [LastWeek, LastMonth, LastQuarter, LastYear, AllTime] {
        assert_eq!(storage.save_statistics_timeframe(value).unwrap(), value);
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.statistics_timeframe().unwrap(), value);
    }
    assert!(storage
        .connection
        .execute(
            "INSERT INTO statistics_settings VALUES (2, 'last_year')",
            []
        )
        .is_err());
    assert!(storage
        .connection
        .execute("UPDATE statistics_settings SET timeframe = 'unknown'", [])
        .is_err());
    storage
        .connection
        .execute("DELETE FROM statistics_settings", [])
        .unwrap();
    assert!(matches!(
        storage.statistics_timeframe(),
        Err(StorageError::InvalidSettings)
    ));
    assert!(matches!(
        storage.save_statistics_timeframe(LastWeek),
        Err(StorageError::InvalidSettings)
    ));
}

#[test]
fn statistics_timeframe_write_failure_keeps_previous_value_and_can_retry() {
    use yomoku_core::statistics::StatisticsTimeframe::*;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage.connection.execute_batch("CREATE TRIGGER fail_timeframe BEFORE UPDATE ON statistics_settings BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(matches!(
        storage.save_statistics_timeframe(LastYear),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(storage.statistics_timeframe().unwrap(), LastWeek);
    assert!(storage.connection.is_autocommit());
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_timeframe")
        .unwrap();
    assert_eq!(
        storage.save_statistics_timeframe(LastYear).unwrap(),
        LastYear
    );
    drop(storage);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .statistics_timeframe()
            .unwrap(),
        LastYear
    );
}

#[test]
fn legacy_six_month_preference_slot_is_the_rolling_quarter_without_migration() {
    use yomoku_core::statistics::StatisticsTimeframe::LastQuarter;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute(
            "UPDATE statistics_settings SET timeframe = 'last_six_months'",
            [],
        )
        .unwrap();
    assert_eq!(storage.statistics_timeframe().unwrap(), LastQuarter);
    assert_eq!(storage.schema_version().unwrap(), 2);
    storage.save_statistics_timeframe(LastQuarter).unwrap();
    let slot: String = storage
        .connection
        .query_row("SELECT timeframe FROM statistics_settings", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(slot, "last_six_months");
    drop(storage);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .statistics_timeframe()
            .unwrap(),
        LastQuarter
    );
}
