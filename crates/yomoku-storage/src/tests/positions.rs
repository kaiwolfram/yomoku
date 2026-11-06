use yomoku_core::{
    document::{ChapterId, DocumentId, PasteText, PastedDocument, SourceId, TextOffset},
    pagination::ReadingPosition,
};

use super::*;

pub(super) fn position(document: &PastedDocument, byte: usize) -> ReadingPosition {
    ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: TextOffset::at(&document.chapter.text, byte).unwrap(),
    }
}

#[test]
fn saves_current_offset_in_both_directions_with_exact_unicode_and_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&PasteText::new("𠮷\0か\u{3099}\r\n終".into()).unwrap())
        .unwrap();
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    for (byte, percent) in [(16, 100), (5, 31), (4, 25), (0, 0), (11, 68)] {
        let expected = position(&document, byte);
        let saved = storage
            .save_document_reading_position(document.document_id, expected)
            .map(|value| value.map(crate::tests::as_paste))
            .unwrap()
            .unwrap();
        assert_eq!(saved.reading_position, Some(expected));
        assert_eq!(saved.chapter, document.chapter);
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(
            storage.load_paste(document.document_id).unwrap(),
            Some(saved)
        );
        assert_eq!(
            storage.list_documents().unwrap()[0].progress_percent,
            percent
        );
    }
}

#[test]
fn rejects_foreign_out_of_range_and_split_scalar_positions_preserving_previous_row() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    let previous = position(&document, 4);
    let expected = storage
        .save_document_reading_position(document.document_id, previous)
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap();
    for invalid in [
        ReadingPosition {
            source_id: SourceId::try_from(i64::MAX).unwrap(),
            ..previous
        },
        ReadingPosition {
            chapter_id: ChapterId::try_from(i64::MAX).unwrap(),
            ..previous
        },
        ReadingPosition {
            offset: TextOffset::try_from(1).unwrap(),
            ..previous
        },
        ReadingPosition {
            offset: TextOffset::try_from(i64::MAX).unwrap(),
            ..previous
        },
    ] {
        assert!(matches!(
            storage
                .save_document_reading_position(document.document_id, invalid)
                .map(|value| value.map(crate::tests::as_paste)),
            Err(StorageError::InvalidDocument)
        ));
        assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    }
    assert!(storage
        .save_document_reading_position(DocumentId::try_from(i64::MAX).unwrap(), previous)
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap()
        .is_none());
}

#[test]
fn insert_update_and_commit_failures_leave_previous_position_and_allow_retry() {
    for (stage, trigger) in [
        ("insert", "CREATE TRIGGER fail_position BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("update", "CREATE TRIGGER fail_position BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("commit", "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_position AFTER UPDATE ON reading_positions BEGIN INSERT INTO deferred_failure VALUES (-1); END;"),
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage.unanalyzed_fixture(&PasteText::new("𠮷本文".into()).unwrap()).unwrap();
        let expected = if stage == "insert" { document.clone() } else {
            storage.save_document_reading_position(document.document_id, position(&document, 4)).map(|value| value.map(crate::tests::as_paste)).unwrap().unwrap()
        };
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.save_document_reading_position(document.document_id, position(&document, 7)).map(|value| value.map(crate::tests::as_paste)), Err(StorageError::Sqlite(_))), "{stage}");
        assert_eq!(storage.load_paste(document.document_id).unwrap(), Some(expected));
        storage.connection.execute_batch("DROP TRIGGER fail_position").unwrap();
        assert!(storage.save_document_reading_position(document.document_id, position(&document, 7)).map(|value| value.map(crate::tests::as_paste)).unwrap().is_some());
    }
}

#[test]
fn corrupt_position_is_rejected_without_repair_and_document_deletion_removes_only_progress() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = storage
        .unanalyzed_fixture(&PasteText::new("𠮷本文".into()).unwrap())
        .unwrap();
    storage
        .save_document_reading_position(document.document_id, position(&document, 4))
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap();
    storage
        .connection
        .execute_batch("UPDATE reading_positions SET byte_offset = 1")
        .unwrap();
    assert!(matches!(
        storage.load_paste(document.document_id),
        Err(StorageError::InvalidDocument)
    ));
    assert_eq!(
        storage
            .connection
            .query_row("SELECT byte_offset FROM reading_positions", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        1
    );
    storage
        .connection
        .execute_batch("DELETE FROM documents")
        .unwrap();
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM reading_positions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM sources", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
