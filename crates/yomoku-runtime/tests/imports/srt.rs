use super::*;

#[test]
fn srt_typed_decoding_errors_skip_full_and_manual_choices_retry_without_partial_files() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let before = srt_totals(&database.path());
    for (bytes, encoding, error) in [
        (&[0xC2, 0xA9][..], None, DocumentError::SrtEncodingRequired),
        (
            &[0x82, 0xA0],
            Some(TextEncoding::Utf8),
            DocumentError::SrtInvalidUtf8,
        ),
        (
            &[0xFF],
            Some(TextEncoding::ShiftJis),
            DocumentError::SrtInvalidShiftJis,
        ),
        (
            &[0xFF, 0xFE, 0x41, 0],
            None,
            DocumentError::SrtUnsupportedEncoding,
        ),
        (&[0xFF], None, DocumentError::SrtUndecodable),
        (b" \r\n", None, DocumentError::EmptySrt),
        (
            b"1\n00:00:01,000 --> 00:00:02,000\n<i> </i>",
            None,
            DocumentError::EmptySrt,
        ),
    ] {
        let rejected = dispatch(&router, srt_event(bytes, encoding));
        assert_eq!(rejected.document_error, Some(error));
        assert!(rejected.documents.is_empty() && rejected.import_summary.is_none());
        assert_eq!(srt_totals(&database.path()), before);
    }
    let mut blank = match srt_event(b"valid", None) {
        Event::ImportSrt(input) => input,
        _ => unreachable!(),
    };
    blank.filename = " \r\n".into();
    assert_eq!(
        dispatch(&router, Event::ImportSrt(blank)).document_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(trace.try_recv().is_err());
    let selected = dispatch(
        &router,
        srt_event(&[0xC2, 0xA9], Some(TextEncoding::ShiftJis)),
    );
    assert_eq!(
        selected.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Created
    );
    let mut storage = Storage::open(database.path()).unwrap();
    let id = selected.documents[0].document_id;
    assert_eq!(storage.load_paste(id).unwrap().unwrap().chapter.text, "ﾂｩ");
    assert_eq!(
        storage_fixture::srt_file(&rusqlite::Connection::open(database.path()).unwrap(), id)
            .unwrap()
            .unwrap()
            .encoding,
        "shift_jis"
    );
    let unicode = dispatch(
        &router,
        srt_event("猫".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert!(unicode.document_error.is_none());
    let legacy = dispatch(&router, srt_event(&[0x82, 0xA0], None));
    assert!(legacy.document_error.is_none());
    assert_eq!(
        storage
            .load_paste(legacy.import_summary.unwrap().document_id)
            .unwrap()
            .unwrap()
            .chapter
            .text,
        "あ"
    );
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn srt_duplicate_of_txt_skips_decoding_and_unavailable_full() {
    let database = Database::new();
    let bytes = b"1\n00:00:01,000 --> 00:00:02,000\n<i>English</i>";
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let created = dispatch(&router, txt_event(bytes, None));
    let id = created.import_summary.unwrap().document_id;
    finish_worker(router, lifecycle, &trace);
    let before = srt_totals(&database.path());
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let duplicate = dispatch(&router, srt_event(bytes, Some(TextEncoding::ShiftJis)));
    let summary = duplicate.import_summary.unwrap();
    assert_eq!(summary.document_id, id);
    assert_eq!(summary.outcome, yomoku_core::app::ImportOutcome::Existing);
    assert_eq!(duplicate.documents, created.documents);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .load_paste(id)
            .unwrap()
            .unwrap()
            .chapter
            .text
            .as_bytes(),
        bytes
    );
    assert!(storage.load_txt_source(id).unwrap().is_some());
    assert!(
        storage_fixture::srt_file(&rusqlite::Connection::open(database.path()).unwrap(), id)
            .unwrap()
            .is_none()
    );
    assert_eq!(srt_totals(&database.path()), before);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn srt_loader_analysis_and_invalid_output_failures_preserve_completed_files_and_retry() {
    let database = Database::new();
    let Factory {
        loader,
        trace,
        failure,
    } = Factory::new(true, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    for mode in [0, 1, 2] {
        failure.store(mode, Ordering::Relaxed);
        let failed = dispatch(
            &router,
            srt_event("猫 𠮷".as_bytes(), Some(TextEncoding::Utf8)),
        );
        assert_eq!(
            failed.document_error,
            Some(if mode == 2 {
                DocumentError::InvalidDocument
            } else {
                DocumentError::Unavailable
            })
        );
        assert_eq!(srt_totals(&database.path()), vec![0; 14]);
    }
    failure.store(0, Ordering::Relaxed);
    let created = dispatch(
        &router,
        srt_event("猫 𠮷".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert!(created.document_error.is_none());
    let before = srt_totals(&database.path());
    failure.store(1, Ordering::Relaxed);
    let rejected = dispatch(
        &router,
        srt_event("犬".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert_eq!(rejected.documents, created.documents);
    assert!(rejected.import_summary.is_none());
    assert_eq!(srt_totals(&database.path()), before);
    failure.store(0, Ordering::Relaxed);
    assert_eq!(
        dispatch(
            &router,
            srt_event("犬".as_bytes(), Some(TextEncoding::Utf8))
        )
        .documents
        .len(),
        2
    );
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn srt_each_write_and_commit_failure_is_atomic_through_worker() {
    for table in [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "imported_files",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "commit",
    ] {
        let database = Database::new();
        let Factory { loader, trace, .. } = Factory::new(false, None);
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let created = dispatch(&router, srt_event(b"English", None));
        let before = srt_totals(&database.path());
        let connection = Connection::open(database.path()).unwrap();
        if table == "commit" {
            connection.execute_batch("CREATE TABLE deferred_parent (id INTEGER PRIMARY KEY);
                CREATE TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TRIGGER reject_import AFTER INSERT ON vocabulary_status_events
                BEGIN INSERT INTO deferred_child VALUES (99); END;").unwrap();
        } else {
            connection.execute_batch(&format!("CREATE TRIGGER reject_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        let bytes = "｜猫《ねこ》 𠮷".as_bytes();
        let failed = dispatch(&router, srt_event(bytes, Some(TextEncoding::Utf8)));
        assert_eq!(
            failed.document_error,
            Some(DocumentError::Unavailable),
            "{table}"
        );
        assert_eq!(failed.documents, created.documents);
        assert_eq!(srt_totals(&database.path()), before, "{table}");
        connection
            .execute_batch("DROP TRIGGER reject_import")
            .unwrap();
        assert_eq!(
            dispatch(&router, srt_event(bytes, Some(TextEncoding::Utf8)))
                .documents
                .len(),
            2
        );
        finish_worker(router, lifecycle, &trace);
    }
}

fn srt_totals(path: &PathBuf) -> Vec<i64> {
    file_totals(path)
}

#[test]
fn srt_duplicates_preserve_first_interpretation_status_position_and_skip_full_after_reopen() {
    let database = Database::new();
    let bytes = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,000\r\n<i>猫</i> 猫\r\n\r\n2\r\n00:00:02,000 --> 00:00:03,000\r\n｜𠮷《よし》".as_bytes();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let created = dispatch(&router, srt_event(bytes, None));
    assert_eq!(created.destination, Destination::Library);
    assert_eq!(created.content_route, ContentRoute::Main);
    assert!(created.reader.is_none());
    let id = created.import_summary.unwrap().document_id;
    assert_eq!(created.documents[0].title, "本𠮷.srt");
    let opened = dispatch(&router, Event::OpenReader(id));
    assert!(matches!(
        opened.reader_analysis,
        Some(ReaderAnalysis::Available(_))
    ));
    let position = yomoku_core::pagination::ReadingPosition {
        offset: TextOffset::try_from(3).unwrap(),
        ..opened.reader.unwrap().position
    };
    let saved = dispatch(&router, Event::SaveReadingPosition(position));
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = Surface::new("猫").unwrap();
    let entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 123)
        .unwrap();
    let source =
        storage_fixture::srt_file(&rusqlite::Connection::open(database.path()).unwrap(), id)
            .unwrap()
            .unwrap();
    assert_eq!(source.original_bytes, bytes);
    assert_eq!(
        yomoku_import::srt::parse(std::str::from_utf8(bytes).unwrap())
            .unwrap()
            .cues,
        vec![0..7, 9..28]
    );
    assert_eq!(
        storage.load_paste(id).unwrap().unwrap().chapter.text,
        "猫 猫\n\n｜𠮷《よし》"
    );
    let before = srt_totals(&database.path());
    lifecycle.set_foreground(false).unwrap();
    // Refresh is a FIFO barrier after cache release.
    dispatch(&router, Event::RefreshLibrary);
    let duplicate = Event::ImportSrt(SrtImport {
        original_bytes: bytes.to_vec(),
        filename: String::new(),
        encoding: Some(TextEncoding::ShiftJis),
    });
    let existing = dispatch(&router, duplicate.clone());
    assert_eq!(
        existing.import_summary.unwrap().outcome,
        yomoku_core::app::ImportOutcome::Existing
    );
    assert_eq!(existing.documents, saved.documents);
    finish_worker(router, lifecycle, &trace);
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    assert_eq!(
        dispatch(&router, duplicate)
            .import_summary
            .unwrap()
            .document_id,
        id
    );
    // Cross-format duplicate references the first canonical interpretation.
    assert_eq!(
        dispatch(&router, txt_event(bytes, Some(TextEncoding::ShiftJis)))
            .import_summary
            .unwrap()
            .document_id,
        id
    );
    assert_eq!(
        storage.load_paste(id).unwrap().unwrap().reading_position,
        Some(position)
    );
    assert_eq!(
        storage_fixture::srt_file(&rusqlite::Connection::open(database.path()).unwrap(), id)
            .unwrap(),
        Some(source)
    );
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), known);
    assert_eq!(srt_totals(&database.path()), before);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}
