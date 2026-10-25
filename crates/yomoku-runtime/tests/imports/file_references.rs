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

fn complete(router: &EffectRouter, timestamp: i64) -> ViewModel {
    let page = router.view().reader.unwrap();
    let (sender, receiver) = mpsc::channel();
    router.update(
        Event::CompleteReaderPage {
            document_id: page.document_id,
            page: page.anchor,
            position: page.position,
            completed_at_unix_ms: timestamp,
        },
        move |_| {
            let _ = sender.send(());
        },
    );
    loop {
        receiver.recv_timeout(TIMEOUT).unwrap();
        let view = router.view();
        if !view.storage_busy {
            assert_eq!(view.document_error, None);
            assert!(!view.completion_save_pending);
            return view;
        }
    }
}

#[test]
fn ordered_reader_worker_crosses_chapters_with_original_analysis_time_and_whole_book_finish() {
    use yomoku_core::{
        active_time::{PendingReadingTime, ReadingTimeUpdate},
        reader::ReaderTokenKey,
    };
    let database = Database::new();
    let bytes = epub_fixture::book(true, "<p>猫 𠮷 English</p>");
    let mut storage = Storage::open(database.path()).unwrap();
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(&bytes, "chapters.epub", 1, |chapter| {
            Ok::<_, DocumentError>(fixture(chapter))
        })
        .unwrap()
    else {
        panic!("new EPUB")
    };
    let id = imported.document.document_id;
    let original = storage.load_document_reader_snapshot(id).unwrap().unwrap();
    assert_eq!(original.document.chapters.len(), 2);
    assert_ne!(original.analyses[0].analysis, original.analyses[1].analysis);
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let (router, lifecycle) = effect_router_with_analyzed_imports(
        database.path(),
        Box::new(move || {
            called.fetch_add(1, Ordering::SeqCst);
            Err(DocumentError::Unavailable)
        }),
    )
    .unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let opened = dispatch(&router, Event::OpenReader(id));
    assert_eq!(opened.loaded_document, Some(original.document.clone()));
    assert_eq!(opened.reader_analysis, Some(original.analyses[0].analysis));
    assert_eq!(opened.reader.as_ref().unwrap().page_number, 1);
    assert_eq!(opened.reader.as_ref().unwrap().total_pages, 2);
    let first = opened.reader.unwrap();
    let first_key = ReaderTokenKey {
        document_id: id,
        page: first.anchor,
        analysis: original.analyses[0].analysis,
    };
    assert!(!dispatch(&router, Event::LoadReaderTokens(first_key))
        .reader_tokens
        .unwrap()
        .tokens
        .is_empty());
    let restored = dispatch(&router, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    dispatch(
        &router,
        Event::CheckpointReadingTime(ReadingTimeUpdate {
            pending: PendingReadingTime::from_millis(123).unwrap(),
            ..restored
        }),
    );
    let moved = complete(&router, 10);
    let second = moved.reader.unwrap();
    assert_eq!(moved.content_route, ContentRoute::Reader);
    assert!(moved.book_completion_notice.is_none());
    assert_eq!(second.page_number, 2);
    assert_eq!(
        second.position.chapter_id,
        original.document.chapters[1].chapter_id
    );
    assert_eq!(second.position.offset, TextOffset::START);
    assert_eq!(moved.reader_analysis, Some(original.analyses[1].analysis));
    assert!(moved.reader_time.is_none() && moved.reader_tokens.is_none());
    assert!(moved.documents[0].progress_percent > 0 && moved.documents[0].progress_percent < 100);
    assert!(!moved.documents[0].finished_at_least_once);
    let before_previous = totals(&database.path());
    let back = dispatch(
        &router,
        Event::PreviousReaderPage {
            document_id: id,
            page: second.anchor,
            position: second.position,
        },
    );
    assert_eq!(back.reader.as_ref().unwrap().position, first.position);
    assert_eq!(back.reader_analysis, Some(original.analyses[0].analysis));
    assert_eq!(back.documents[0].progress_percent, 0);
    assert_eq!(totals(&database.path()), before_previous);
    let reread = complete(&router, 11);
    assert_eq!(reread.reader_analysis, Some(original.analyses[1].analysis));
    // Reflow preserves an exact later-chapter UTF-8 position, not just page start.
    let position = ReadingPosition {
        offset: TextOffset::at(&original.document.chapters[1].text, "猫 ".len()).unwrap(),
        ..second.position
    };
    let saved = dispatch(&router, Event::SaveReadingPosition(position));
    let progress = saved.documents[0].progress_percent;
    router.update(Event::OpenSettings, |_| {});
    for length in [
        yomoku_core::pagination::PageLength::Characters250,
        yomoku_core::pagination::PageLength::Characters750,
        yomoku_core::pagination::PageLength::Characters1000,
        yomoku_core::pagination::PageLength::Characters1500,
        yomoku_core::pagination::PageLength::Characters500,
    ] {
        let reflowed = dispatch(&router, Event::SetPageLength(length));
        assert_eq!(reflowed.reader.as_ref().unwrap().position, position);
        assert_eq!(reflowed.reader.as_ref().unwrap().total_pages, 2);
        assert_eq!(
            reflowed.reader_analysis,
            Some(original.analyses[1].analysis)
        );
        assert_eq!(reflowed.documents[0].progress_percent, progress);
    }
    router.update(Event::CloseSettings, |_| {});
    let second_key = ReaderTokenKey {
        document_id: id,
        page: second.anchor,
        analysis: original.analyses[1].analysis,
    };
    let tokens = dispatch(&router, Event::LoadReaderTokens(second_key))
        .reader_tokens
        .unwrap();
    assert!(tokens
        .tokens
        .iter()
        .all(|token| token.anchor.chapter_id == second.position.chapter_id));
    assert!(tokens.tokens.iter().any(|token| token.surface == "猫"));
    let restored = dispatch(&router, Event::RestoreReadingTime)
        .reader_time
        .unwrap();
    assert_eq!(restored.pending.milliseconds(), 0);
    dispatch(
        &router,
        Event::CheckpointReadingTime(ReadingTimeUpdate {
            pending: PendingReadingTime::from_millis(456).unwrap(),
            ..restored
        }),
    );
    let finished = complete(&router, 12);
    assert_eq!(finished.content_route, ContentRoute::Main);
    assert!(finished.reader.is_none() && finished.book_completion_notice.is_some());
    assert_eq!(finished.documents[0].progress_percent, 100);
    assert!(finished.documents[0].finished_at_least_once);
    assert_eq!(
        finished.loaded_document.as_ref().unwrap().chapters,
        original.document.chapters
    );
    let durations: Vec<i64> = Connection::open(database.path())
        .unwrap()
        .prepare("SELECT active_time_ms FROM reading_completions ORDER BY command_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(durations, vec![123, 0, 456]);
    // Ordinary loads retain chapter-2 EOF; a fresh worker explicitly opens at chapter 1.
    let loaded = dispatch(&router, Event::LoadPaste(id))
        .loaded_document
        .unwrap();
    assert_eq!(
        loaded.reading_position.unwrap().chapter_id,
        second.position.chapter_id
    );
    drop(router);
    drop(lifecycle);
    let (router, lifecycle) = effect_router_with_analyzed_imports(
        database.path(),
        Box::new(|| Err(DocumentError::Unavailable)),
    )
    .unwrap();
    let reopened = dispatch(&router, Event::OpenReader(id));
    assert_eq!(reopened.reader.as_ref().unwrap().position, first.position);
    assert_eq!(
        reopened.reader_analysis,
        Some(original.analyses[0].analysis)
    );
    assert!(reopened.reader.unwrap().finished_at_least_once);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    drop(router);
    drop(lifecycle);
}

#[test]
fn image_only_chapter_worker_can_load_tokens_and_finish_without_words() {
    use yomoku_core::reader::ReaderTokenKey;
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<img src='../images/cover.png'/>"),
            "images.epub",
            1,
            |chapter| {
                let mut analysis = fixture(chapter);
                for token in &mut analysis.tokens {
                    if token.surface == "\u{fffc}" {
                        token.is_japanese = false;
                    }
                }
                Ok::<_, DocumentError>(analysis)
            },
        )
        .unwrap()
    else {
        panic!("new EPUB")
    };
    let id = imported.document.document_id;
    let (router, lifecycle) = effect_router_with_analyzed_imports(
        database.path(),
        Box::new(|| Err(DocumentError::Unavailable)),
    )
    .unwrap();
    dispatch(&router, Event::RefreshLibrary);
    dispatch(&router, Event::OpenReader(id));
    let view = complete(&router, 10);
    let page = view.reader.unwrap();
    assert_eq!(page.text, "\u{fffc}");
    let tokens = dispatch(
        &router,
        Event::LoadReaderTokens(ReaderTokenKey {
            document_id: id,
            page: page.anchor,
            analysis: view.reader_analysis.unwrap(),
        }),
    );
    assert!(tokens.reader_tokens.unwrap().tokens.is_empty());
    dispatch(&router, Event::RestoreReadingTime);
    let finished = complete(&router, 11);
    assert!(finished.book_completion_notice.is_some());
    assert!(finished.completion_notice.is_none());
    assert!(finished.documents[0].finished_at_least_once);
    drop(router);
    drop(lifecycle);
}

#[test]
fn corrupt_ordered_reader_load_fails_without_truncation_or_analysis_and_next_reader_opens() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>猫</p>"),
            "corrupt.epub",
            1,
            |chapter| Ok::<_, DocumentError>(fixture(chapter)),
        )
        .unwrap()
    else {
        panic!("new EPUB")
    };
    let valid = storage
        .create_paste(&PasteText::new("later 𠮷".into()).unwrap())
        .unwrap();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute(
            "UPDATE epub_documents SET chapter_count = chapter_count + 1",
            [],
        )
        .unwrap();
    let before = totals(&database.path());
    let (router, lifecycle) = effect_router_with_analyzed_imports(
        database.path(),
        Box::new(|| panic!("reader must not load Full")),
    )
    .unwrap();
    let rejected = dispatch(&router, Event::OpenReader(imported.document.document_id));
    assert_eq!(
        rejected.document_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(rejected.reader.is_none() && rejected.loaded_document.is_none());
    let opened = dispatch(&router, Event::OpenReader(valid.document_id));
    assert_eq!(opened.document_error, None);
    assert_eq!(opened.loaded_document.unwrap(), valid.into());
    assert_eq!(opened.reader_analysis, Some(ReaderAnalysis::Missing));
    assert_eq!(totals(&database.path()), before);
    drop(router);
    drop(lifecycle);
}
