use super::*;
use yomoku_core::{document::PasteText, library::LibrarySort};

fn create(storage: &mut Storage, text: &str) -> yomoku_core::document::PastedDocument {
    storage
        .create_paste(&PasteText::new(text.into()).unwrap())
        .unwrap()
}

fn clock(storage: &Storage) -> i64 {
    storage
        .connection
        .query_row("SELECT sequence FROM library_read_clock", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn recency_changes_only_on_successful_explicit_entry_and_survives_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = create(&mut storage, "first\n本文");
    let second = create(&mut storage, "second\n本文");
    assert!(storage
        .list_documents()
        .unwrap()
        .iter()
        .all(|document| document.last_read_sequence.is_none()));
    assert_eq!(clock(&storage), 0);
    let first_open = storage
        .open_document_reader_snapshot(first.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(first_open.last_read_sequence, Some(1));
    storage
        .open_reader_snapshot(second.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        storage.last_read_sequence(second.document_id).unwrap(),
        Some(2)
    );
    let latest = storage
        .open_document_reader_snapshot(first.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(latest.last_read_sequence, Some(3));
    // Explicit opening preserves position/finish/content; ordinary loads and
    // movement never update recency or reset an opened document to unknown.
    assert_eq!(first_open.document, latest.document);
    storage
        .save_document_reading_position(
            first.document_id,
            latest.document.current_position().unwrap(),
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for _ in 0..2 {
        let loaded = storage
            .load_document_reader_snapshot(first.document_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.last_read_sequence, Some(3));
        storage.load_reader_snapshot(second.document_id).unwrap();
        let listed = storage.list_documents().unwrap();
        assert_eq!(
            LibrarySort::LastRead.project(&listed, "")[0].document_id,
            first.document_id
        );
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(clock(&storage), 3);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.last_read_sequence(first.document_id).unwrap(),
        Some(3)
    );
    assert_eq!(
        storage
            .open_document_reader_snapshot(second.document_id)
            .unwrap()
            .unwrap()
            .last_read_sequence,
        Some(4)
    );
    assert_eq!(
        storage
            .open_document_reader_snapshot(999.try_into().unwrap())
            .unwrap(),
        None
    );
    assert_eq!(clock(&storage), 4);
}

#[test]
fn failed_reader_entry_rolls_back_recency_clock_and_finish_reset_then_retries() {
    for trigger in [
        "CREATE TRIGGER fail BEFORE UPDATE ON library_read_clock BEGIN SELECT RAISE(ABORT, 'clock'); END;",
        "CREATE TRIGGER fail BEFORE UPDATE OF last_read_sequence ON documents BEGIN SELECT RAISE(ABORT, 'recency'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED);
         CREATE TRIGGER fail AFTER UPDATE OF last_read_sequence ON documents BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = create(&mut storage, "𠮷\n本文");
        let position = yomoku_core::pagination::ReadingPosition {
            source_id: document.chapter.source_id, chapter_id: document.chapter.chapter_id,
            offset: yomoku_core::document::TextOffset::at(&document.chapter.text, document.chapter.text.len()).unwrap(),
        };
        storage.save_document_reading_position(document.document_id, position).unwrap();
        storage.connection.execute("UPDATE documents SET finished_at_least_once = 1", []).unwrap();
        let before = storage.load_document_reader_snapshot(document.document_id).unwrap().unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(storage.open_document_reader_snapshot(document.document_id).is_err());
        assert_eq!(clock(&storage), 0);
        assert_eq!(storage.load_document_reader_snapshot(document.document_id).unwrap(), Some(before));
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        let after = storage.open_document_reader_snapshot(document.document_id).unwrap().unwrap();
        assert_eq!(after.last_read_sequence, Some(1));
        assert_eq!(after.document.current_position().unwrap().offset, yomoku_core::document::TextOffset::START);
        assert!(after.document.finished_at_least_once);
        assert_eq!(clock(&storage), 1);
    }
}

#[test]
fn concurrent_opens_allocate_distinct_recency_and_reimport_starts_unopened() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let documents: Vec<_> = (0..4)
        .map(|index| create(&mut storage, &format!("book {index}\n本文")))
        .collect();
    let barrier = Arc::new(Barrier::new(documents.len()));
    let mut sequences = std::thread::scope(|scope| {
        let handles: Vec<_> = documents
            .iter()
            .map(|document| {
                let barrier = barrier.clone();
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .open_document_reader_snapshot(document.document_id)
                        .unwrap()
                        .unwrap()
                        .last_read_sequence
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    sequences.sort();
    assert_eq!(sequences, vec![1, 2, 3, 4]);
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(documents[0].document_id)],
        )
        .unwrap();
    let fresh = create(&mut storage, &documents[0].chapter.text);
    assert!(fresh.document_id > documents[3].document_id);
    assert_eq!(storage.last_read_sequence(fresh.document_id).unwrap(), None);
    assert_eq!(clock(&storage), 4);
    assert_eq!(
        storage
            .open_document_reader_snapshot(fresh.document_id)
            .unwrap()
            .unwrap()
            .last_read_sequence,
        Some(5)
    );
}

#[test]
fn exhausted_clock_fails_atomically_without_reusing_order() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = create(&mut storage, "book\n本文");
    storage
        .connection
        .execute("UPDATE library_read_clock SET sequence = ?1", [i64::MAX])
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.open_document_reader_snapshot(document.document_id),
        Err(StorageError::InvalidDocument)
    ));
    assert_eq!(
        storage.last_read_sequence(document.document_id).unwrap(),
        None
    );
    assert_eq!(clock(&storage), i64::MAX);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn exact_file_duplicates_keep_read_recency_across_formats_without_writes_or_analysis() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = "猫".as_bytes();
    let id = match storage
        .import_analyzed_txt(
            bytes,
            "first.txt",
            Some(yomoku_import::txt::TextEncoding::Utf8),
            1,
            super::imports::fixture,
        )
        .unwrap()
    {
        TxtImportOutcome::Created(imported) => imported.document.document_id,
        _ => panic!("new import"),
    };
    storage.open_document_reader_snapshot(id).unwrap().unwrap();
    let expected = storage.list_documents().unwrap().remove(0);
    assert_eq!(expected.last_read_sequence, Some(1));
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage.find_imported_file(bytes).unwrap(),
        Some(expected.clone())
    );
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    let no_analysis = |_: &yomoku_core::document::ChapterText| -> Result<
        yomoku_core::analysis::ChapterAnalysis,
        std::convert::Infallible,
    > { panic!("duplicate must not analyze") };
    assert!(
        matches!(storage.import_analyzed_txt(bytes, "again.txt", None, 2, no_analysis).unwrap(), TxtImportOutcome::Existing(document) if document == expected)
    );
    assert!(
        matches!(storage.import_analyzed_srt(bytes, "again.srt", None, 2, no_analysis).unwrap(), SrtImportOutcome::Existing(document) if document == expected)
    );
    assert!(
        matches!(storage.import_analyzed_epub(bytes, "again.epub", 2, no_analysis).unwrap(), EpubImportOutcome::Existing(document) if document == expected)
    );
    assert_eq!(clock(&storage), 1);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}
