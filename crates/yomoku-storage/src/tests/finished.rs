use yomoku_core::{
    completion::AnalyzedCompletionOutcome,
    document::{DocumentId, PasteText, PastedDocument},
    pagination::PageLength,
    reader::ReaderPage,
};

use super::{positions::position, *};

fn paste(storage: &mut Storage, text: &str) -> PastedDocument {
    storage
        .analyzed_fixture(
            &PasteText::new(text.to_owned()).unwrap(),
            0,
            super::imports::fixture,
        )
        .unwrap()
        .document
}

fn command(
    storage: &mut Storage,
    document: &PastedDocument,
) -> yomoku_core::effective_reader::EffectivePageCompletion {
    let snapshot = storage
        .load_document_reader_snapshot(document.document_id)
        .unwrap()
        .unwrap();
    let page =
        yomoku_core::reader::ReaderPage::open_document(&snapshot.document, snapshot.page_length)
            .unwrap();
    let yomoku_core::reader::ReaderAnalysis::Available(analysis_id) = snapshot.analyses[0].analysis
    else {
        panic!("fixture analysis")
    };
    storage
        .prepare_fixture_completion(yomoku_core::completion::PageCompletionIntent {
            document_id: document.document_id,
            page: page.anchor,
            page_length: snapshot.page_length,
            departing_position: page.position,
            completed_at_unix_ms: 500,
            analysis_id,
        })
        .unwrap()
}

#[test]
fn books_without_read_override_inherit_completion_and_keep_saved_positions() {
    use crate::documents::{create_paste_in, record_reader_entry_in, write_position_in};
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    let connection = &storage.connection;
    let partial = create_paste_in(connection, &PasteText::new("猫犬".into()).unwrap()).unwrap();
    let finished = create_paste_in(connection, &PasteText::new("犬猫".into()).unwrap()).unwrap();
    write_position_in(connection, partial.document_id, position(&partial, 3)).unwrap();
    write_position_in(connection, finished.document_id, position(&finished, 6)).unwrap();
    record_reader_entry_in(connection, partial.document_id).unwrap();
    connection
        .execute(
            "UPDATE documents SET finished_at_least_once = 1 WHERE id = ?1",
            [i64::from(finished.document_id)],
        )
        .unwrap();
    drop(storage);

    let mut storage = Storage::open(database.path()).unwrap();
    let summaries = storage.list_documents().unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].document_id, partial.document_id);
    assert_eq!(summaries[0].progress_percent, 50);
    assert_eq!(summaries[0].last_read_sequence, Some(1));
    assert!(!summaries[0].is_read);
    assert_eq!(summaries[1].document_id, finished.document_id);
    assert_eq!(summaries[1].progress_percent, 100);
    assert!(summaries[1].is_read);
    for id in [partial.document_id, finished.document_id] {
        assert_eq!(storage.load_paste(id).unwrap().unwrap().read_override, None);
    }
    assert!(storage
        .connection
        .execute("UPDATE documents SET read_override = 2", [])
        .is_err());
}

#[test]
fn manual_read_status_survives_restart_and_replay_until_a_new_finish() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, &format!("{}\n終。", "猫".repeat(500)));
    let id = document.document_id;
    let partial = position(&document, "猫".repeat(500).len() + 1);
    storage.save_document_reading_position(id, partial).unwrap();
    let before = storage.list_documents().unwrap().remove(0);
    assert!(storage.set_document_read(id, true).unwrap());
    let mut marked = before;
    marked.is_read = true;
    assert_eq!(storage.list_documents().unwrap(), [marked]);
    drop(storage);

    let mut storage = Storage::open(database.path()).unwrap();
    let opened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    assert_eq!(opened.document.reading_position, Some(partial));
    assert!(opened.document.library_summary().unwrap().is_read);
    assert!(!opened.document.finished_at_least_once);
    let first = command(&mut storage, &document);
    storage.record_effective_page_completion(&first).unwrap();
    assert_eq!(storage.load_paste(id).unwrap().unwrap().read_override, None);
    let mut expected = storage.list_documents().unwrap().remove(0);
    assert!(expected.is_read);
    assert_eq!(expected.progress_percent, 100);

    storage
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_read_override BEFORE UPDATE OF read_override ON documents
         BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        )
        .unwrap();
    assert!(matches!(
        storage.set_document_read(id, false),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(storage.list_documents().unwrap(), [expected.clone()]);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_read_override")
        .unwrap();
    assert!(storage.set_document_read(id, false).unwrap());
    expected.is_read = false;
    assert!(!storage
        .set_document_read(DocumentId::try_from(i64::MAX).unwrap(), true)
        .unwrap());
    drop(storage);

    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.list_documents().unwrap(), [expected.clone()]);
    assert_eq!(
        storage.record_effective_page_completion(&first).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(storage.list_documents().unwrap(), [expected]);
    let opened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    assert!(opened.document.finished_at_least_once);
    assert!(!opened.document.library_summary().unwrap().is_read);
    assert_eq!(
        opened.document.reading_position,
        Some(position(&document, 0))
    );
    let moved = storage
        .save_document_reading_position(id, partial)
        .unwrap()
        .unwrap();
    assert!(!moved.library_summary().unwrap().is_read);
    let reread = command(&mut storage, &document);
    storage.record_effective_page_completion(&reread).unwrap();
    assert!(storage.list_documents().unwrap()[0].is_read);
    assert_eq!(storage.load_paste(id).unwrap().unwrap().read_override, None);
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn finished_marker_survives_backward_position_reflow_reopen_and_each_reread() {
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
    storage.record_effective_page_completion(&first).unwrap();
    for length in PageLength::ALL {
        storage.save_page_length(length).unwrap();
        let previous = storage
            .save_document_reading_position(document.document_id, position(&document, 0))
            .map(|value| value.map(crate::tests::as_paste))
            .unwrap()
            .unwrap();
        assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
        assert_eq!(
            storage
                .finished_at_least_once(document.document_id)
                .unwrap(),
            Some(true)
        );

        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.page_length().unwrap(), length);
        let before = storage.load_paste(document.document_id).unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&first).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        assert_eq!(storage.load_paste(document.document_id).unwrap(), before);
        assert_eq!(
            storage
                .finished_at_least_once(document.document_id)
                .unwrap(),
            Some(true)
        );
        let reread = command(&mut storage, &previous);
        assert_ne!(
            reread.completion().command_id,
            first.completion().command_id
        );
        assert_eq!(
            storage.record_effective_page_completion(&reread).unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
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
    assert_eq!(count, 1 + PageLength::ALL.len() as i64);
    assert!(matches!(
        storage
            .connection
            .execute("UPDATE reading_completions SET page_length = 1500", []),
        Err(rusqlite::Error::SqliteFailure(error, _))
            if error.code == rusqlite::ErrorCode::ConstraintViolation
    ));
}

#[test]
fn position_at_eof_without_completion_never_marks_finished() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = paste(&mut storage, "猫𠮷\0終");
    storage
        .save_document_reading_position(
            document.document_id,
            position(&document, document.chapter.text.len()),
        )
        .map(|value| value.map(crate::tests::as_paste))
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
    storage
        .record_effective_page_completion(&completion)
        .unwrap();
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
        storage
            .record_effective_page_completion(&completion)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
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
                let completion = completion.clone();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    let outcome = storage
                        .record_effective_page_completion(&completion)
                        .unwrap();
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
            .filter(|&&o| o == AnalyzedCompletionOutcome::Recorded { newly_known: 1 })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|&&o| o == AnalyzedCompletionOutcome::AlreadyRecorded)
            .count(),
        3
    );
}

#[test]
fn explicit_finished_open_resets_durably_but_loads_and_completion_reloads_keep_position() {
    use yomoku_core::reader::ReaderAnalysis;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .analyzed_fixture(
            &PasteText::new(format!("{}\n終𠮷\0", "猫".repeat(1600))).unwrap(),
            1,
            super::imports::fixture,
        )
        .unwrap();
    let document = imported.document;
    let id = document.document_id;
    let eof = position(&document, document.chapter.text.len());
    let unfinished = storage
        .save_document_reading_position(id, eof)
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap()
        .unwrap();
    // Position-only EOF must still reopen exactly, without a checkmark.
    assert_eq!(
        storage
            .open_document_reader_snapshot(id)
            .unwrap()
            .unwrap()
            .document,
        unfinished.clone().into()
    );
    let final_page = command(&mut storage, &unfinished);
    storage
        .record_effective_page_completion(&final_page)
        .unwrap();
    let completed = storage.load_paste(id).unwrap().unwrap();
    assert!(completed.finished_at_least_once);
    assert_eq!(completed.reading_position, Some(eof));
    assert_eq!(
        storage
            .load_document_reader_snapshot(id)
            .unwrap()
            .unwrap()
            .document,
        completed.clone().into()
    );
    assert!(storage.list_documents().unwrap()[0].is_read);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);

    for length in PageLength::ALL {
        storage.save_page_length(length).unwrap();
        let partial = position(&document, 3);

        // A finished reread can save a partial-page position before reopening at page 1.
        storage
            .save_document_reading_position(id, partial)
            .map(|value| value.map(crate::tests::as_paste))
            .unwrap();

        let before = storage.load_paste(id).unwrap().unwrap();
        assert_eq!(
            storage
                .load_document_reader_snapshot(id)
                .unwrap()
                .unwrap()
                .document,
            before.clone().into()
        );
        let opened = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        assert_eq!(opened.page_length, length);
        assert_eq!(
            opened.analyses[0].analysis,
            ReaderAnalysis::Available(imported.analysis_id)
        );
        assert_eq!(
            opened.document.reading_position,
            Some(position(&document, 0))
        );
        assert!(opened.document.finished_at_least_once);
        let page = ReaderPage::open_document(&opened.document, length).unwrap();
        assert_eq!(page.page_number, 1);
        assert!(page.finished_at_least_once);
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(
            storage.load_paste(id).unwrap(),
            Some(crate::tests::as_paste(opened.document))
        );

        let summary = &storage.list_documents().unwrap()[0];
        assert!(summary.is_read);
        assert_eq!(summary.progress_percent, 0);
        assert_eq!(
            storage
                .record_effective_page_completion(&final_page)
                .unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
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
        .open_document_reader_snapshot(DocumentId::try_from(i64::MAX).unwrap())
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
        storage.record_effective_page_completion(&completion).unwrap();
        let before = storage.load_document_reader_snapshot(document.document_id).unwrap().unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(matches!(storage.open_document_reader_snapshot(document.document_id), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.load_document_reader_snapshot(document.document_id).unwrap(), Some(before));
        storage.connection.execute_batch("DROP TRIGGER fail_open").unwrap();
        let opened = storage.open_document_reader_snapshot(document.document_id).unwrap().unwrap();
        assert_eq!(opened.document.reading_position, Some(position(&document, 0)));
        assert!(opened.document.finished_at_least_once);
        // Once at page 1, duplicate opens need no additional position write.
        storage.connection.execute_batch("CREATE TRIGGER fail_open BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        let reopened = storage.open_document_reader_snapshot(document.document_id).unwrap().unwrap();
        assert_eq!(reopened.document, opened.document);
        assert!(reopened.last_read_sequence > opened.last_read_sequence);
    }
}
