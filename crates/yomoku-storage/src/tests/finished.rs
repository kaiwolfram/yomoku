use yomoku_core::{
    active_time::PendingReadingTime,
    completion::{CompletionOutcome, PageCompletion},
    document::{DocumentId, PasteText, PastedDocument},
    pagination::PageLength,
    reader::ReaderPage,
};

use super::{positions::position, *};

fn paste(storage: &mut Storage, text: &str) -> PastedDocument {
    storage
        .create_paste(&PasteText::new(text.to_owned()).unwrap())
        .unwrap()
}

fn command(storage: &mut Storage, document: &PastedDocument) -> PageCompletion {
    let page_length = storage.page_length().unwrap();
    let page = ReaderPage::open(document, page_length).unwrap();
    PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page: page.anchor,
        page_length,
        departing_position: page.position,
        completed_at_unix_ms: 500,
        pending_time: storage
            .pending_page_time(document.document_id, page.anchor, page_length)
            .unwrap(),
    }
}

// Seed precisely the pre-marker history shape without running the newer writer
// against an old schema. This is not a production completion path.
fn old_completion(storage: &mut Storage, document: &PastedDocument) -> PageCompletion {
    let page_length = storage.page_length().unwrap();
    let page = ReaderPage::open(document, page_length).unwrap();
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page: page.anchor,
        page_length,
        departing_position: page.position,
        completed_at_unix_ms: 500,
        pending_time: Default::default(),
    };
    storage
        .connection
        .execute(
            "INSERT INTO reading_completions VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, 500, 500, 0)",
            (
                i64::from(completion.command_id),
                i64::from(document.document_id),
                i64::from(document.chapter.source_id),
                i64::from(document.chapter.chapter_id),
                i64::from(completion.page.range.start),
                i64::from(completion.page.range.end),
                i64::from(completion.departing_position.offset),
            ),
        )
        .unwrap();
    completion
}

#[test]
fn finished_marker_survives_backward_position_reflow_time_reopen_and_each_reread() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, "猫𠮷\0終");
    assert_eq!(
        storage
            .finished_at_least_once(document.document_id)
            .unwrap(),
        Some(false)
    );
    let first = command(&mut storage, &document);
    storage.record_page_completion(first).unwrap();
    for length in [PageLength::Characters250, PageLength::Characters1500] {
        storage.save_page_length(length).unwrap();
        let previous = storage
            .save_reading_position(document.document_id, position(&document, 0))
            .unwrap()
            .unwrap();
        assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
        assert_eq!(
            storage
                .finished_at_least_once(document.document_id)
                .unwrap(),
            Some(true)
        );
        let token = storage
            .load_pending_reading_time(document.document_id, position(&document, 0))
            .unwrap();
        storage
            .save_pending_reading_time(token, PendingReadingTime::from_millis(123).unwrap())
            .unwrap();
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        let before = storage.load_paste(document.document_id).unwrap();
        assert_eq!(
            storage.record_page_completion(first).unwrap(),
            CompletionOutcome::AlreadyRecorded
        );
        assert_eq!(storage.load_paste(document.document_id).unwrap(), before);
        assert_eq!(
            storage
                .finished_at_least_once(document.document_id)
                .unwrap(),
            Some(true)
        );
        let reread = command(&mut storage, &previous);
        assert_ne!(reread.command_id, first.command_id);
        assert_eq!(
            storage.record_page_completion(reread).unwrap(),
            CompletionOutcome::Recorded
        );
        assert_eq!(
            storage
                .finished_at_least_once(document.document_id)
                .unwrap(),
            Some(true)
        );
    }
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn position_at_eof_without_completion_never_marks_finished() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, "猫𠮷\0終");
    storage
        .save_reading_position(
            document.document_id,
            position(&document, document.chapter.text.len()),
        )
        .unwrap();
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);
    assert_eq!(
        storage
            .finished_at_least_once(document.document_id)
            .unwrap(),
        Some(false)
    );
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .finished_at_least_once(document.document_id)
            .unwrap(),
        Some(false)
    );
    assert_eq!(
        storage
            .finished_at_least_once(DocumentId::try_from(i64::MAX).unwrap())
            .unwrap(),
        None
    );
}

#[test]
fn deletion_reimport_and_late_replay_preserve_history_without_restoring_marker() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, "猫𠮷\0終");
    let completion = command(&mut storage, &document);
    storage.record_page_completion(completion).unwrap();
    // No production deletion API is introduced by this repository slice.
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(document.chapter.chapter_id)],
        )
        .unwrap();
    let reimport = paste(&mut storage, &document.chapter.text);
    assert_ne!(reimport.document_id, document.document_id);
    assert_eq!(
        storage.record_page_completion(completion).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .finished_at_least_once(document.document_id)
            .unwrap(),
        None
    );
    assert_eq!(
        storage
            .finished_at_least_once(reimport.document_id)
            .unwrap(),
        Some(false)
    );
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn concurrent_final_callbacks_commit_one_history_row_and_one_marker() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, "猫𠮷\0終");
    let completion = command(&mut storage, &document);
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    let outcome = storage.record_page_completion(completion).unwrap();
                    assert_eq!(
                        storage
                            .finished_at_least_once(document.document_id)
                            .unwrap(),
                        Some(true)
                    );
                    outcome
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results
            .iter()
            .filter(|&&o| o == CompletionOutcome::Recorded)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|&&o| o == CompletionOutcome::AlreadyRecorded)
            .count(),
        3
    );
}

#[test]
fn migration_backfills_only_live_document_eof_history_and_preserves_existing_state() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..10]).unwrap();
    let mut old = Storage { connection };
    // Embedded NUL and supplementary Unicode require byte length, not SQL text length.
    let finished = paste(&mut old, "猫𠮷\0終");
    let incomplete = paste(&mut old, &format!("{}\n終", "語".repeat(500)));
    let eof_only = paste(&mut old, "文");
    let deleted = paste(&mut old, "削除");
    let finished_command = old_completion(&mut old, &finished);
    old_completion(&mut old, &incomplete);
    old_completion(&mut old, &deleted);
    old.connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(deleted.document_id)],
        )
        .unwrap();
    let reimport = paste(&mut old, &deleted.chapter.text);
    let mut finished = legacy_position(&old, &finished, position(&finished, 0)).unwrap();
    finished.finished_at_least_once = true;
    let eof_only = legacy_position(
        &old,
        &eof_only,
        position(&eof_only, eof_only.chapter.text.len()),
    )
    .unwrap();
    let token = yomoku_core::active_time::SavedReadingTime {
        value: yomoku_core::active_time::ReadingTimeUpdate {
            document_id: finished.document_id,
            position: position(&finished, 0),
            pending: PendingReadingTime::from_millis(1234).unwrap(),
        },
        revision: 2,
    };
    old.connection
        .execute(
            "INSERT INTO pending_reading_time VALUES (?1, ?2, 0, 1234, 2)",
            (
                i64::from(finished.document_id),
                i64::from(finished.chapter.chapter_id),
            ),
        )
        .unwrap();
    old.save_page_length(PageLength::Characters750).unwrap();
    let expected: Vec<_> = [&finished, &incomplete, &eof_only, &reimport]
        .map(|document| document.library_summary().unwrap())
        .into();
    drop(old);
    for _ in 0..2 {
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 22);
        assert_eq!(storage.list_documents().unwrap(), expected);
        assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
        assert_eq!(
            storage
                .finished_at_least_once(finished.document_id)
                .unwrap(),
            Some(true)
        );
        for document in [&incomplete, &eof_only, &reimport] {
            assert_eq!(
                storage
                    .finished_at_least_once(document.document_id)
                    .unwrap(),
                Some(false)
            );
        }
        assert_eq!(
            storage.finished_at_least_once(deleted.document_id).unwrap(),
            None
        );
        assert_eq!(
            storage
                .load_pending_reading_time(finished.document_id, position(&finished, 0))
                .unwrap(),
            token
        );
        assert_eq!(
            storage.record_page_completion(finished_command).unwrap(),
            CompletionOutcome::AlreadyRecorded
        );
        let count: i64 = storage
            .connection
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
        let integrity: String = storage
            .connection
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
    }
}

#[test]
fn failed_backfill_rolls_back_column_history_and_data_then_retries() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..10]).unwrap();
    let mut old = Storage { connection };
    let document = paste(&mut old, "猫𠮷\0終");
    old_completion(&mut old, &document);
    let expected = Some(PastedDocument {
        finished_at_least_once: true,
        ..document.clone()
    });
    old.connection.execute_batch("CREATE TRIGGER fail_finish BEFORE UPDATE ON documents BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    drop(old);
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    let connection = Connection::open(database.path()).unwrap();
    let has_column: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('documents') WHERE name = 'finished_at_least_once')", [], |r| r.get(0)).unwrap();
    assert!(!has_column);
    assert_eq!(history(&database.path()).len(), 10);
    connection
        .execute_batch("DROP TRIGGER fail_finish")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    assert_eq!(
        storage
            .finished_at_least_once(document.document_id)
            .unwrap(),
        Some(true)
    );
}

#[test]
fn explicit_finished_open_resets_durably_but_loads_and_completion_reloads_keep_position() {
    use yomoku_core::reader::ReaderAnalysis;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new(format!("{}\n終𠮷\0", "猫".repeat(1600))).unwrap(),
            1,
            super::imports::fixture,
        )
        .unwrap();
    let document = imported.document;
    let id = document.document_id;
    let eof = position(&document, document.chapter.text.len());
    let unfinished = storage.save_reading_position(id, eof).unwrap().unwrap();
    // Position-only EOF must still reopen exactly, without a checkmark.
    assert_eq!(
        storage.open_reader_snapshot(id).unwrap().unwrap().document,
        unfinished
    );
    let final_page = command(&mut storage, &unfinished);
    storage.record_page_completion(final_page).unwrap();
    let completed = storage.load_paste(id).unwrap().unwrap();
    assert!(completed.finished_at_least_once);
    assert_eq!(completed.reading_position, Some(eof));
    assert_eq!(
        storage.load_reader_snapshot(id).unwrap().unwrap().document,
        completed
    );
    assert!(storage.list_documents().unwrap()[0].finished_at_least_once);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);

    for length in PageLength::ALL {
        storage.save_page_length(length).unwrap();
        let partial = position(&document, 3);
        let token = storage.load_pending_reading_time(id, partial).unwrap();
        // A finished reread's partial-page time survives closing/reopening at page 1.
        storage.save_reading_position(id, partial).unwrap();
        let token = storage
            .save_pending_reading_time(token, PendingReadingTime::from_millis(123).unwrap())
            .unwrap();
        let before = storage.load_paste(id).unwrap().unwrap();
        assert_eq!(
            storage.load_reader_snapshot(id).unwrap().unwrap().document,
            before
        );
        let opened = storage.open_reader_snapshot(id).unwrap().unwrap();
        assert_eq!(opened.page_length, length);
        assert_eq!(
            opened.analysis,
            ReaderAnalysis::Available(imported.analysis_id)
        );
        assert_eq!(
            opened.document.reading_position,
            Some(position(&document, 0))
        );
        assert!(opened.document.finished_at_least_once);
        let page = ReaderPage::open(&opened.document, length).unwrap();
        assert_eq!(page.page_number, 1);
        assert!(page.finished_at_least_once);
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.load_paste(id).unwrap(), Some(opened.document));
        assert_eq!(
            storage.load_pending_reading_time(id, partial).unwrap(),
            token
        );
        let summary = &storage.list_documents().unwrap()[0];
        assert!(summary.finished_at_least_once);
        assert_eq!(summary.progress_percent, 0);
        assert_eq!(
            storage.record_page_completion(final_page).unwrap(),
            CompletionOutcome::AlreadyRecorded
        );
        assert_eq!(
            storage.load_paste(id).unwrap().unwrap().reading_position,
            Some(position(&document, 0))
        );
    }
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(storage
        .open_reader_snapshot(DocumentId::try_from(i64::MAX).unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn finished_open_write_or_commit_failure_preserves_snapshot_and_retry_resets_once() {
    for trigger in [
        "CREATE TRIGGER fail_open BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_open AFTER UPDATE ON reading_positions BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = paste(&mut storage, "猫𠮷\0終");
        let completion = command(&mut storage, &document);
        storage.record_page_completion(completion).unwrap();
        let before = storage.load_reader_snapshot(document.document_id).unwrap().unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.open_reader_snapshot(document.document_id), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.load_reader_snapshot(document.document_id).unwrap(), Some(before));
        storage.connection.execute_batch("DROP TRIGGER fail_open").unwrap();
        let opened = storage.open_reader_snapshot(document.document_id).unwrap().unwrap();
        assert_eq!(opened.document.reading_position, Some(position(&document, 0)));
        assert!(opened.document.finished_at_least_once);
        // Once at page 1, duplicate opens need no additional position write.
        storage.connection.execute_batch("CREATE TRIGGER fail_open BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        assert_eq!(storage.open_reader_snapshot(document.document_id).unwrap(), Some(opened));
    }
}
