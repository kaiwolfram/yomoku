use super::*;
use yomoku_core::{app::ImportOutcome, pagination::ReadingPosition};
use yomoku_storage::EpubImportOutcome;

#[path = "../../../yomoku-import/tests/support/epub.rs"]
mod epub_fixture;

fn import_event(bytes: &[u8], srt: bool) -> Event {
    // Existing bytes must win before blank-name/invalid-encoding validation.
    if srt {
        Event::ImportSrt(SrtImport {
            original_bytes: bytes.to_vec(),
            filename: String::new(),
            encoding: Some(TextEncoding::ShiftJis),
        })
    } else {
        Event::ImportTxt(TxtImport {
            original_bytes: bytes.to_vec(),
            filename: String::new(),
            encoding: Some(TextEncoding::ShiftJis),
        })
    }
}

#[test]
fn txt_and_srt_reference_complete_epub_without_loading_full_or_resetting_progress() {
    let database = Database::new();
    let bytes = epub_fixture::book(true, "<p>猫 𠮷 English</p>");
    let mut storage = Storage::open(database.path()).unwrap();
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(&bytes, "first.epub", 1, |chapter| {
            Ok::<_, DocumentError>(fixture(chapter))
        })
        .unwrap()
    else {
        panic!("expected new EPUB")
    };
    let id = imported.document.document_id;
    let chapter = &imported.document.chapters[1].chapter;
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::at(&chapter.text, "猫 ".len()).unwrap(),
    };
    storage
        .save_document_reading_position(id, position)
        .unwrap();
    Connection::open(database.path())
        .unwrap()
        .execute(
            "UPDATE documents SET title = 'renamed 📖', finished_at_least_once = 1 WHERE id = ?1",
            [i64::from(id)],
        )
        .unwrap();
    let before = storage.load_epub_document(id).unwrap().unwrap();
    let expected = storage.list_documents().unwrap();
    assert!(expected[0].progress_percent > 0 && expected[0].progress_percent < 100);
    let counts = totals(&database.path());
    let file_before = std::fs::read(database.path()).unwrap();

    // Separate fresh workers, with no preceding List and no usable analyzer.
    for srt in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let called = calls.clone();
        let loader: AnalyzerLoader = Box::new(move || {
            called.fetch_add(1, Ordering::SeqCst);
            Err(DocumentError::Unavailable)
        });
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        let view = dispatch(&router, import_event(&bytes, srt));
        assert_eq!(view.document_error, None);
        assert_eq!(view.documents, expected);
        let summary = view.import_summary.unwrap();
        assert_eq!(summary.document_id, id);
        assert_eq!(summary.title, "renamed 📖");
        assert_eq!(summary.outcome, ImportOutcome::Existing);
        assert_eq!(view.content_route, ContentRoute::Main);
        assert_eq!(view.destination, Destination::Library);
        assert!(view.reader.is_none() && view.loaded_document.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(totals(&database.path()), counts);
        assert_eq!(storage.load_epub_document(id).unwrap().unwrap(), before);
        drop(router);
        drop(lifecycle);
    }
    assert_eq!(std::fs::read(database.path()).unwrap(), file_before);
}

#[test]
fn corrupt_existing_epub_fails_without_analysis_and_later_file_still_imports() {
    let database = Database::new();
    let bytes = epub_fixture::book(true, "<p>猫</p>");
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .import_analyzed_epub(&bytes, "bad.epub", 1, |chapter| {
            Ok::<_, DocumentError>(fixture(chapter))
        })
        .unwrap();
    Connection::open(database.path())
        .unwrap()
        .execute(
            "UPDATE epub_documents SET chapter_count = chapter_count + 1",
            [],
        )
        .unwrap();
    let counts = totals(&database.path());
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    for srt in [false, true] {
        let rejected = dispatch(&router, import_event(&bytes, srt));
        assert_eq!(
            rejected.document_error,
            Some(DocumentError::InvalidDocument)
        );
        assert!(rejected.documents.is_empty() && rejected.import_summary.is_none());
        assert!(rejected.reader.is_none());
        assert!(trace.try_recv().is_err());
        assert_eq!(totals(&database.path()), counts);
    }
    let imported = dispatch(
        &router,
        Event::ImportTxt(TxtImport {
            original_bytes: b"English".to_vec(),
            filename: "good.txt".into(),
            encoding: None,
        }),
    );
    assert_eq!(imported.document_error, None);
    assert_eq!(
        imported.import_summary.unwrap().outcome,
        ImportOutcome::Created
    );
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Load(_)
    ));
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Analyze(_)
    ));
    assert_eq!(totals(&database.path())[0], counts[0] + 1);
    finish_worker(router, lifecycle, &trace);
}
