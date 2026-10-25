use super::*;
use yomoku_core::{app::ImportOutcome, imports::EpubImport};

pub(super) fn event(bytes: &[u8]) -> Event {
    Event::ImportEpub(EpubImport {
        original_bytes: bytes.to_vec(),
        filename: "本𠮷.epub".into(),
    })
}

fn counts(path: &PathBuf) -> Vec<i64> {
    let mut counts = file_totals(path);
    let connection = Connection::open(path).unwrap();
    for table in [
        "epub_documents",
        "epub_chapters",
        "epub_assets",
        "epub_spans",
        "epub_inline_images",
        "sqlite_sequence",
    ] {
        counts.push(
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap(),
        );
    }
    counts
}

#[test]
fn whole_book_import_registers_all_chapters_without_encounters_and_reopens_without_full() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    let first = dispatch(
        &router,
        txt_event("猫".as_bytes(), Some(TextEncoding::Utf8)),
    );
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    let cat = storage
        .set_vocabulary_status(cat.id, VocabularyStatus::Familiar, 1)
        .unwrap()
        .unwrap();
    let bytes = epub_fixture::book(true, "<p>猫 犬 犬 𠮷 English</p>");
    let parsed = yomoku_import::epub::parse(&bytes).unwrap();
    let created = dispatch(&router, event(&bytes));
    assert_eq!(created.document_error, None);
    assert_eq!(created.content_route, ContentRoute::Main);
    assert_eq!(created.destination, Destination::Library);
    assert!(created.reader.is_none() && created.loaded_document.is_none());
    let summary = created.import_summary.unwrap();
    assert_eq!(summary.outcome, ImportOutcome::Created);
    assert_eq!(summary.title, parsed.title.unwrap());
    assert_eq!(created.documents.len(), first.documents.len() + 1);
    let stored = storage
        .load_epub_document(summary.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.original_bytes, bytes);
    assert_eq!(stored.images, parsed.images);
    assert_eq!(stored.chapters.len(), 2);
    let snapshot = storage
        .load_document_reader_snapshot(summary.document_id)
        .unwrap()
        .unwrap();
    for ((stored, parsed), original) in stored
        .chapters
        .iter()
        .zip(parsed.chapters)
        .zip(snapshot.analyses)
    {
        assert_eq!(stored.chapter.text, parsed.text);
        assert_eq!(stored.ruby, parsed.ruby);
        assert_eq!(stored.spans, parsed.spans);
        assert_eq!(stored.images, parsed.images);
        let ReaderAnalysis::Available(id) = original.analysis else {
            panic!("missing original B")
        };
        assert_eq!(
            storage.chapter_analysis(id).unwrap().unwrap(),
            fixture(&stored.chapter)
        );
    }
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap(),
        Some(cat)
    );
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new("犬").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    let connection = Connection::open(database.path()).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(connection.query_row("SELECT COUNT(*) FROM vocabulary_status_events WHERE vocabulary_id = (SELECT id FROM vocabulary WHERE surface = '犬')", [], |row| row.get::<_, i64>(0)).unwrap(), 1);
    finish_worker(router, lifecycle, &trace);
    let (router, lifecycle) = effect_router_with_analyzed_imports(
        database.path(),
        Box::new(|| panic!("open cannot load Full")),
    )
    .unwrap();
    let opened = dispatch(&router, Event::OpenReader(summary.document_id));
    assert_eq!(
        opened.loaded_document.unwrap().chapters,
        stored
            .chapters
            .into_iter()
            .map(|chapter| chapter.chapter)
            .collect::<Vec<_>>()
    );
    assert_eq!(opened.reader.unwrap().total_pages, 2);
    drop(router);
    drop(lifecycle);
}

#[test]
fn invalid_containers_skip_full_and_later_valid_file_imports_with_filename_fallback() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let before = counts(&database.path());
    let invalid_xml = epub_fixture::book(true, "<p>unclosed");
    for bytes in [
        b"".as_slice(),
        &[0x82, 0xA0],
        &[0xFF, 0xFE, 0x41, 0],
        b"PK truncated ZIP",
        &invalid_xml,
    ] {
        let failed = dispatch(&router, event(bytes));
        assert_eq!(failed.document_error, Some(DocumentError::InvalidEpub));
        assert!(failed.import_summary.is_none() && failed.documents.is_empty());
        assert_eq!(counts(&database.path()), before);
    }
    assert!(trace.try_recv().is_err());
    let valid = epub_fixture::book(false, "<p>猫</p>");
    let rejected = dispatch(
        &router,
        Event::ImportEpub(EpubImport {
            original_bytes: valid.clone(),
            filename: " ".into(),
        }),
    );
    assert_eq!(
        rejected.document_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(trace.try_recv().is_err());
    let imported = dispatch(&router, event(&valid));
    assert_eq!(imported.document_error, None);
    assert_eq!(imported.import_summary.unwrap().title, "本𠮷.epub");
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn epub_duplicate_of_txt_or_srt_preserves_first_interpretation_before_container_or_name_checks() {
    for srt in [false, true] {
        let database = Database::new();
        let bytes = b"English first interpretation";
        let Factory { loader, trace, .. } = Factory::new(false, None);
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        let created = dispatch(
            &router,
            if srt {
                srt_event(bytes, None)
            } else {
                txt_event(bytes, None)
            },
        );
        finish_worker(router, lifecycle, &trace);
        let before = counts(&database.path());
        let Factory { loader, trace, .. } = Factory::new(true, None);
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        let duplicate = dispatch(
            &router,
            Event::ImportEpub(EpubImport {
                original_bytes: bytes.to_vec(),
                filename: String::new(),
            }),
        );
        assert_eq!(duplicate.document_error, None);
        assert_eq!(duplicate.documents, created.documents);
        assert_eq!(
            duplicate.import_summary.unwrap().outcome,
            ImportOutcome::Existing
        );
        assert_eq!(counts(&database.path()), before);
        assert!(trace.try_recv().is_err());
        finish_worker(router, lifecycle, &trace);
    }
}

#[test]
fn loader_and_later_chapter_analysis_failures_roll_back_then_next_file_and_retry_succeed() {
    for invalid in [false, true] {
        let database = Database::new();
        let loads = Arc::new(AtomicUsize::new(0));
        let calls = loads.clone();
        let loader: AnalyzerLoader = Box::new(move || {
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(DocumentError::Unavailable);
            }
            let mut chapter_number = 0;
            Ok(Box::new(move |chapter: &ChapterText| {
                chapter_number += 1;
                if chapter_number == 2 {
                    if !invalid {
                        return Err(DocumentError::Unavailable);
                    }
                    let mut analysis = fixture(chapter);
                    analysis.mode = SplitMode::C;
                    return Ok(analysis);
                }
                Ok(fixture(chapter))
            }))
        });
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        dispatch(&router, Event::RefreshLibrary);
        let before = counts(&database.path());
        let bytes = epub_fixture::book(true, "<p>later 猫</p>");
        assert_eq!(
            dispatch(&router, event(&bytes)).document_error,
            Some(DocumentError::Unavailable)
        );
        assert_eq!(counts(&database.path()), before);
        let rejected = dispatch(&router, event(&bytes));
        assert_eq!(
            rejected.document_error,
            Some(if invalid {
                DocumentError::InvalidDocument
            } else {
                DocumentError::Unavailable
            })
        );
        assert_eq!(counts(&database.path()), before);
        // Failing analyzer is evicted; a later file uses a fresh factory.
        assert_eq!(
            dispatch(&router, txt_event(b"English", None)).document_error,
            None
        );
        assert_eq!(loads.load(Ordering::SeqCst), 3);
        // Release the fake's per-instance failure counter before retrying EPUB.
        lifecycle.set_foreground(false).unwrap();
        dispatch(&router, Event::RefreshLibrary);
        let single = epub_fixture::book(false, "<p>other</p>");
        // A healthy independent worker retries the original full book.
        drop(router);
        drop(lifecycle);
        let Factory { loader, trace, .. } = Factory::new(false, None);
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        dispatch(&router, Event::RefreshLibrary);
        assert_eq!(dispatch(&router, event(&bytes)).document_error, None);
        assert_eq!(dispatch(&router, event(&single)).documents.len(), 3);
        finish_worker(router, lifecycle, &trace);
    }
}

#[test]
fn every_epub_write_later_chapter_and_deferred_commit_failure_rolls_back_whole_book() {
    for table in [
        "sources",
        "documents",
        "chapters",
        "chapter_texts",
        "imported_files",
        "epub_documents",
        "epub_assets",
        "epub_chapters",
        "source_ruby",
        "epub_spans",
        "epub_inline_images",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "later_chapter",
        "commit",
    ] {
        let database = Database::new();
        let Factory { loader, trace, .. } = Factory::new(false, None);
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
        let created = dispatch(&router, txt_event(b"English", None));
        let before = counts(&database.path());
        let connection = Connection::open(database.path()).unwrap();
        if table == "commit" {
            connection.execute_batch("CREATE TABLE deferred_parent (id INTEGER PRIMARY KEY); CREATE TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER reject_import AFTER INSERT ON epub_chapters BEGIN INSERT INTO deferred_child VALUES (99); END;").unwrap();
        } else if table == "later_chapter" {
            connection.execute_batch("CREATE TRIGGER reject_import AFTER INSERT ON chapters WHEN NEW.chapter_index = 1 BEGIN SELECT RAISE(ABORT, 'later chapter'); END;").unwrap();
        } else {
            connection.execute_batch(&format!("CREATE TRIGGER reject_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        let bytes = epub_fixture::book(true, "<p>犬 𠮷</p>");
        let rejected = dispatch(&router, event(&bytes));
        assert_eq!(
            rejected.document_error,
            Some(DocumentError::Unavailable),
            "{table}"
        );
        assert_eq!(rejected.documents, created.documents);
        assert!(rejected.import_summary.is_none());
        assert_eq!(counts(&database.path()), before, "{table}");
        connection
            .execute_batch("DROP TRIGGER reject_import")
            .unwrap();
        let retry = dispatch(&router, event(&bytes));
        assert_eq!(retry.document_error, None, "{table}");
        assert_eq!(retry.documents.len(), 2);
        assert_eq!(i64::from(retry.import_summary.unwrap().document_id), 2);
        finish_worker(router, lifecycle, &trace);
    }
}

#[test]
fn cancel_in_later_chapter_discards_earlier_chapter_assets_registrations_and_allows_retry() {
    let database = Database::new();
    let (entered, ready) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let mut gate = Some(gate);
    let loader: AnalyzerLoader = Box::new(move || {
        let mut chapter_number = 0;
        let gate = gate.take().unwrap();
        let entered = entered.clone();
        Ok(Box::new(move |chapter: &ChapterText| {
            chapter_number += 1;
            if chapter_number == 2 {
                entered.send(()).unwrap();
                gate.recv_timeout(TIMEOUT).unwrap();
            }
            Ok(fixture(chapter))
        }))
    });
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    // Seed a committed file without using this analyzer.
    Storage::open(database.path())
        .unwrap()
        .create_paste(&PasteText::new("prior".into()).unwrap())
        .unwrap();
    let prior = dispatch(&router, Event::RefreshLibrary).documents;
    let before = counts(&database.path());
    let bytes = epub_fixture::book(true, "<p>犬 𠮷</p>");
    let (done, result) = mpsc::channel();
    router.update(event(&bytes), move |_| {
        done.send(()).unwrap();
    });
    ready.recv_timeout(TIMEOUT).unwrap();
    assert!(router
        .update(event(&bytes), |_| panic!("duplicate busy event accepted"))
        .is_empty());
    lifecycle.cancel_file_import().unwrap();
    assert!(router.view().storage_busy && result.try_recv().is_err());
    assert_eq!(counts(&database.path()), before);
    release.send(()).unwrap();
    result.recv_timeout(TIMEOUT).unwrap();
    let cancelled = router.view();
    assert_eq!(cancelled.documents, prior);
    assert!(cancelled.document_error.is_none() && cancelled.import_summary.is_none());
    assert_eq!(counts(&database.path()), before);
    let retry = dispatch(&router, event(&bytes));
    assert_eq!(retry.document_error, None);
    assert_eq!(retry.documents.len(), 2);
    lifecycle.cancel_file_import().unwrap();
    assert_eq!(
        dispatch(&router, event(&bytes))
            .import_summary
            .unwrap()
            .outcome,
        ImportOutcome::Existing
    );
}

fn wrapped_event(bytes: &[u8], _: Option<TextEncoding>) -> Event {
    event(&epub_fixture::book(
        true,
        &format!("<p>{}</p>", std::str::from_utf8(bytes).unwrap()),
    ))
}

#[test]
fn cancel_reaches_queued_epub_before_fifo_starts() {
    cancel_reaches_queued_attempt_before_fifo_can_start_it(wrapped_event);
}

#[test]
fn cancel_remains_available_during_closed_router_epub_drain() {
    cancel_remains_available_while_closed_router_drains_accepted_analysis(wrapped_event);
}
