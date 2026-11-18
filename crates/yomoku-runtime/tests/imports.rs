#[path = "../../../tests/support/storage.rs"]
mod storage_fixture;
use std::{
    path::PathBuf,
    rc::Rc,
    sync::{
        atomic::{AtomicU8, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::{self, ThreadId},
    time::Duration,
};

use crux_core::middleware::Layer;
use rusqlite::Connection;
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::{ContentRoute, Destination, Event, ViewModel},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    imports::{SrtImport, TextEncoding, TxtImport},
    reader::ReaderAnalysis,
    storage::DocumentError,
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_runtime::{AnalyzerLifecycle, AnalyzerLoader, EffectRouter, PasteAnalyzer, ShellEffect};
use yomoku_storage::Storage;

fn import_router(
    path: PathBuf,
    loader: AnalyzerLoader,
) -> std::io::Result<(EffectRouter, AnalyzerLifecycle)> {
    let word_path = path.with_extension("words.sqlite3");
    // Synthetic analyzer fixtures have identical C/B/A boundaries. Real Full
    // fixtures delegate component analysis to the actual analyzer below.
    struct Components {
        inner: Box<dyn PasteAnalyzer>,
        last: Option<ChapterAnalysis>,
    }
    impl PasteAnalyzer for Components {
        fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
            let result = self.inner.analyze(chapter)?;
            self.last = Some(result.clone());
            Ok(result)
        }
        fn analyze_mode(
            &mut self,
            chapter: &ChapterText,
            mode: SplitMode,
            profile: &AnalysisProfile,
        ) -> Result<ChapterAnalysis, DocumentError> {
            if profile.engine_revision.starts_with("sudachi.rs@") {
                return self.inner.analyze_mode(chapter, mode, profile);
            }
            let mut result = self.last.clone().ok_or(DocumentError::Unavailable)?;
            result.mode = mode;
            result.profile = profile.clone();
            Ok(result)
        }
    }
    let mut loader = loader;
    yomoku_runtime::effect_router(
        path,
        Box::new(move || {
            Ok(Box::new(Components {
                inner: loader()?,
                last: None,
            }))
        }),
        Box::new(move || {
            if !word_path.exists() {
                let connection =
                    Connection::open(&word_path).map_err(|_| DocumentError::Unavailable)?;
                connection
                    .execute_batch(
                        "PRAGMA application_id = 1498367049; PRAGMA user_version = 1;
                CREATE TABLE entries(source TEXT, entry_id INTEGER, payload TEXT);
                CREATE TABLE forms(surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER);
                CREATE INDEX forms_by_surface ON forms(surface);",
                    )
                    .map_err(|_| DocumentError::Unavailable)?;
            }
            Ok(word_path.clone())
        }),
    )
}

const TIMEOUT: Duration = Duration::from_secs(10);

#[path = "imports/refinement.rs"]
mod refinement;

#[path = "imports/document_deletion.rs"]
mod document_deletion;
#[path = "imports/document_rename.rs"]
mod document_rename;
#[path = "imports/statistics.rs"]
mod statistics;
#[path = "imports/vocabulary.rs"]
mod vocabulary;

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "yomoku-worker-import-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> PathBuf {
        self.0.join("import.sqlite3")
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn dispatch(router: &EffectRouter, event: Event) -> ViewModel {
    dispatch_with_timeout(router, event, TIMEOUT)
}

fn dispatch_with_timeout(router: &EffectRouter, event: Event, timeout: Duration) -> ViewModel {
    let (sender, receiver) = mpsc::channel();
    let initial = router.update(event, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, ShellEffect::Render(_))));
        let _ = sender.send(thread::current().id());
    });
    assert!(initial
        .iter()
        .all(|effect| matches!(effect, ShellEffect::Render(_))));
    assert_ne!(
        receiver.recv_timeout(timeout).unwrap(),
        thread::current().id()
    );
    let view = router.view();
    assert!(!view.storage_busy);
    view
}

#[derive(Debug, PartialEq)]
enum Trace {
    Load(ThreadId),
    Analyze(ThreadId),
    Drop(ThreadId),
}

struct FakeAnalyzer {
    // Deliberately !Send: factory creation must happen inside the worker.
    _local: Rc<()>,
    worker: ThreadId,
    trace: mpsc::Sender<Trace>,
    failure: Arc<AtomicU8>,
    gate: Option<mpsc::Receiver<()>>,
}
impl PasteAnalyzer for FakeAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        assert_eq!(thread::current().id(), self.worker);
        self.trace.send(Trace::Analyze(self.worker)).unwrap();
        if let Some(gate) = self.gate.take() {
            gate.recv_timeout(TIMEOUT).unwrap();
        }
        match self.failure.load(Ordering::Relaxed) {
            1 => Err(DocumentError::Unavailable),
            2 => {
                let mut analysis = fixture(chapter);
                analysis.mode = SplitMode::B;
                Ok(analysis)
            }
            _ => Ok(fixture(chapter)),
        }
    }
}
impl Drop for FakeAnalyzer {
    fn drop(&mut self) {
        assert_eq!(thread::current().id(), self.worker);
        let _ = self.trace.send(Trace::Drop(self.worker));
    }
}

fn fixture(chapter: &ChapterText) -> ChapterAnalysis {
    let mut cursor = 0;
    let tokens = chapter
        .text
        .split_whitespace()
        .map(|surface| {
            let start = cursor + chapter.text[cursor..].find(surface).unwrap();
            cursor = start + surface.len();
            AnalyzedToken {
                anchor: TextAnchor {
                    source_id: chapter.source_id,
                    chapter_id: chapter.chapter_id,
                    range: TextRange {
                        start: TextOffset::at(&chapter.text, start).unwrap(),
                        end: TextOffset::at(&chapter.text, cursor).unwrap(),
                    },
                },
                surface: surface.into(),
                is_japanese: !matches!(surface, "English" | "。" | "📖" | "\0"),
                reading: None,
                dictionary_form: None,
            }
        })
        .collect();
    ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "worker-fixture@1".into(),
            dictionary_revision: "worker-fixture-full@1".into(),
        },
        mode: SplitMode::C,
        tokens,
    }
}

struct Factory {
    loader: AnalyzerLoader,
    trace: mpsc::Receiver<Trace>,
    failure: Arc<AtomicU8>,
}
impl Factory {
    fn new(fail_first_load: bool, mut gate: Option<mpsc::Receiver<()>>) -> Self {
        let (sender, trace) = mpsc::channel();
        let failure = Arc::new(AtomicU8::new(0));
        let state = Arc::clone(&failure);
        let mut first = fail_first_load;
        let loader = Box::new(move || {
            let worker = thread::current().id();
            sender.send(Trace::Load(worker)).unwrap();
            if std::mem::take(&mut first) {
                return Err(DocumentError::Unavailable);
            }
            Ok(Box::new(FakeAnalyzer {
                _local: Rc::new(()),
                worker,
                trace: sender.clone(),
                failure: Arc::clone(&state),
                gate: gate.take(),
            }) as Box<dyn PasteAnalyzer>)
        });
        Self {
            loader,
            trace,
            failure,
        }
    }
}

fn totals(path: &PathBuf) -> Vec<i64> {
    let connection = Connection::open(path).unwrap();
    [
        "documents",
        "sources",
        "chapters",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "paste_content_hashes",
        "reading_positions",
        "reading_completions",
        "completion_commands",
    ]
    .map(|table| {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    })
    .to_vec()
}

fn finish_worker(
    router: EffectRouter,
    lifecycle: AnalyzerLifecycle,
    trace: &mpsc::Receiver<Trace>,
) {
    drop(router);
    // The last callback may still be returning after the test receives its result.
    // Factory disconnection proves the worker drained and released its sender.
    drain_trace(trace);
    assert!(lifecycle.set_foreground(true).is_err());
}

fn drain_trace(trace: &mpsc::Receiver<Trace>) -> Vec<Trace> {
    let mut events = Vec::new();
    loop {
        match trace.recv_timeout(TIMEOUT) {
            Ok(event) => events.push(event),
            Err(mpsc::RecvTimeoutError::Disconnected) => return events,
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("worker did not release its factory"),
        }
    }
}

#[test]
fn reader_open_restores_stored_identity_and_legacy_missing_without_loading_analyzer() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let analyzed = storage_fixture::unanalyzed(
        &mut storage,
        database.path(),
        &PasteText::new("猫 𠮷".into()).unwrap(),
    );
    let identity = storage
        .store_chapter_analysis(analyzed.chapter.chapter_id, &fixture(&analyzed.chapter))
        .unwrap();
    let legacy = storage_fixture::unanalyzed(
        &mut storage,
        database.path(),
        &PasteText::new("昔の本文".into()).unwrap(),
    );
    for _ in 0..2 {
        // fresh worker/session also needs no dictionary
        let Factory { loader, trace, .. } = Factory::new(true, None);
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let before = totals(&database.path());
        for (id, expected) in [
            (analyzed.document_id, ReaderAnalysis::Available(identity)),
            (legacy.document_id, ReaderAnalysis::Missing),
        ] {
            let view = dispatch(&router, Event::OpenReader(id));
            assert!(view.document_error.is_none());
            assert_eq!(view.reader.unwrap().document_id, id);
            assert_eq!(view.reader_analysis, Some(expected));
        }
        assert_eq!(totals(&database.path()), before);
        assert!(trace.try_recv().is_err());
        drop(router);
        assert!(drain_trace(&trace).is_empty());
        assert!(lifecycle.set_foreground(true).is_err());
    }
}

#[test]
fn library_settings_and_invalid_pastes_never_load_analyzer() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    assert!(!database.path().exists());
    assert_eq!(
        dispatch(&router, Event::CreatePaste(" \r\n".into())).document_error,
        Some(DocumentError::EmptyPaste)
    );
    assert!(!database.path().exists());
    assert!(dispatch(&router, Event::RefreshLibrary)
        .documents
        .is_empty());
    router.update(Event::OpenSettings, |_| unreachable!());
    assert!(dispatch(
        &router,
        Event::SetPageLength(yomoku_core::pagination::PageLength::Characters750)
    )
    .document_error
    .is_none());
    lifecycle.set_foreground(false).unwrap();
    lifecycle.set_foreground(true).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn failed_database_open_retries_without_loading_dictionary_first() {
    let database = Database::new();
    let parent = database.0.join("missing");
    let path = parent.join("import.sqlite3");
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(path, loader).unwrap();
    assert_eq!(
        dispatch(&router, Event::CreatePaste("猫".into())).document_error,
        Some(DocumentError::Unavailable)
    );
    assert!(!parent.exists());
    assert!(trace.try_recv().is_err());
    std::fs::create_dir(&parent).unwrap();
    assert!(dispatch(&router, Event::CreatePaste("猫".into()))
        .document_error
        .is_none());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn worker_imports_deduplicate_preserve_status_and_progress_and_reopen() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = Surface::new("猫").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&cat), 10)
        .unwrap();
    let cat_entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(cat_entry.id, VocabularyStatus::Known, 20)
        .unwrap()
        .unwrap();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let text = "猫 猫\r\n𠮷 か\u{3099} English 。 📖 \0";
    let view = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(
        view.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Created
    );
    assert!(view.document_error.is_none());
    assert_eq!(view.content_route, ContentRoute::Main);
    assert!(view.reader.is_none());
    let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
        Trace::Load(id) => id,
        other => panic!("{other:?}"),
    };
    assert_ne!(worker, thread::current().id());
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    let document_id = view.documents[0].document_id;
    let opened = dispatch(&router, Event::OpenReader(document_id))
        .reader
        .unwrap();
    let position = yomoku_core::pagination::ReadingPosition {
        offset: TextOffset::try_from(4).unwrap(),
        ..opened.position
    };
    let saved = dispatch(&router, Event::SaveReadingPosition(position));
    let before = totals(&database.path());
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), Some(known));
    assert_eq!(before[6], 3);
    assert_eq!(before[7], 4); // three registrations plus the manual status event
    assert_eq!(&before[10..], &[0, 0]);
    assert!(before[4] == 1 && before[8] == 1);
    let duplicate = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(
        duplicate.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Existing
    );
    assert_eq!(
        duplicate.import_summary.as_ref().unwrap().document_id,
        document_id
    );
    assert_eq!(duplicate.documents, saved.documents);
    assert!(duplicate.reader.is_none());
    assert!(trace.try_recv().is_err());
    assert_eq!(totals(&database.path()), before);
    assert_eq!(
        storage
            .load_paste(document_id)
            .unwrap()
            .unwrap()
            .reading_position,
        Some(position)
    );
    finish_worker(router, lifecycle, &trace);

    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    // Existing result alone must populate a fresh Library without a preceding list.
    let duplicate = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(
        duplicate.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Existing
    );
    assert_eq!(duplicate.documents, saved.documents);
    assert!(trace.try_recv().is_err()); // Existing pastes never load either asset.
    assert_eq!(totals(&database.path()), before);
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn load_engine_and_invalid_output_failures_roll_back_and_retry_fresh() {
    let database = Database::new();
    let Factory {
        loader,
        trace,
        failure,
    } = Factory::new(true, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::OpenPaste, |_| unreachable!());
    for mode in [0, 1, 2] {
        failure.store(mode, Ordering::Relaxed);
        let rejected = dispatch(&router, Event::CreatePaste("猫 𠮷".into()));
        assert_eq!(
            rejected.document_error,
            Some(if mode == 2 {
                DocumentError::InvalidDocument
            } else {
                DocumentError::Unavailable
            })
        );
        assert_eq!(rejected.content_route, ContentRoute::Paste);
        assert!(rejected.documents.is_empty());
        assert_eq!(totals(&database.path()), vec![0; 12]);
    }
    failure.store(0, Ordering::Relaxed);
    assert!(dispatch(&router, Event::CreatePaste("猫 𠮷".into()))
        .document_error
        .is_none());
    drop(router);
    // The delivered callback can briefly retain its resolver's worker sender.
    // Wait for worker teardown before asserting that the weak handle is closed.
    let events = drain_trace(&trace);
    assert!(lifecycle.set_foreground(false).is_err());
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Trace::Load(_)))
            .count(),
        4
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Trace::Analyze(_)))
            .count(),
        3
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Trace::Drop(_)))
            .count(),
        3
    );
}

#[test]
fn every_write_and_commit_failure_roll_back_through_the_router_and_can_retry() {
    for table in [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "paste_content_hashes",
        "commit",
    ] {
        let database = Database::new();
        Storage::open(database.path()).unwrap();
        let connection = Connection::open(database.path()).unwrap();
        if table == "commit" {
            connection.execute_batch("CREATE TABLE deferred_parent (id INTEGER PRIMARY KEY);
                CREATE TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TRIGGER reject_import AFTER INSERT ON vocabulary_status_events
                BEGIN INSERT INTO deferred_child VALUES (99); END;").unwrap();
        } else {
            connection
                .execute_batch(&format!(
                    "CREATE TRIGGER reject_import AFTER INSERT ON {table}
                BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
                ))
                .unwrap();
        }
        let Factory { loader, trace, .. } = Factory::new(false, None);
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let rejected = dispatch(&router, Event::CreatePaste("猫 𠮷".into()));
        assert_eq!(
            rejected.document_error,
            Some(DocumentError::Unavailable),
            "{table}"
        );
        assert_eq!(totals(&database.path()), vec![0; 12], "{table}");
        connection
            .execute_batch("DROP TRIGGER reject_import")
            .unwrap();
        assert!(dispatch(&router, Event::CreatePaste("猫 𠮷".into()))
            .document_error
            .is_none());
        assert_eq!(totals(&database.path())[6], 2);
        drop(router);
        let events = drain_trace(&trace);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Trace::Load(_)))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Trace::Drop(_)))
                .count(),
            1
        );
        assert!(lifecycle.set_foreground(false).is_err());
    }
}

#[test]
fn background_releases_cache_hidden_imports_release_and_foreground_is_lazy() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::CreatePaste("猫".into()));
    let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
        Trace::Load(id) => id,
        other => panic!("{other:?}"),
    };
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    lifecycle.set_foreground(true).unwrap();
    dispatch(&router, Event::CreatePaste("犬".into()));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    lifecycle.set_foreground(false).unwrap();
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
    lifecycle.set_foreground(false).unwrap();
    dispatch(&router, Event::CreatePaste("鳥".into()));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Load(worker));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
    lifecycle.set_foreground(true).unwrap();
    lifecycle.set_foreground(true).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    assert!(trace.try_recv().is_err());
    dispatch(&router, Event::CreatePaste("猫".into()));
    assert!(trace.try_recv().is_err()); // duplicate did not load or tokenize
    drop(router);
    assert_eq!(
        trace.recv_timeout(TIMEOUT),
        Err(mpsc::RecvTimeoutError::Disconnected)
    );
    assert!(lifecycle.set_foreground(false).is_err());
}

#[test]
fn background_and_close_wait_for_accepted_atomic_work_without_blocking_dispatch() {
    for close in [false, true] {
        let database = Database::new();
        let (release, gate) = mpsc::channel();
        let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let (completed, completion) = mpsc::channel();
        router.update(Event::CreatePaste("猫 𠮷".into()), move |_| {
            let _ = completed.send(());
        });
        let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
            Trace::Load(id) => id,
            other => panic!("{other:?}"),
        };
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
        assert_eq!(totals(&database.path()), vec![0; 12]);
        router.update(
            Event::SelectDestination(Destination::Vocabulary),
            |_| unreachable!(),
        );
        assert_eq!(router.view().destination, Destination::Vocabulary);
        assert!(router.view().storage_busy);
        assert!(router
            .update(
                Event::CreatePaste("duplicate callback".into()),
                |_| unreachable!()
            )
            .is_empty());
        lifecycle.set_foreground(false).unwrap();
        assert!(trace.try_recv().is_err());
        let router = if close {
            drop(router);
            None
        } else {
            Some(router)
        };
        release.send(()).unwrap();
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
        if let Some(router) = router {
            completion.recv_timeout(TIMEOUT).unwrap();
            assert_eq!(router.view().documents.len(), 1);
            finish_worker(router, lifecycle, &trace);
        } else {
            assert_eq!(
                trace.recv_timeout(TIMEOUT),
                Err(mpsc::RecvTimeoutError::Disconnected)
            );
            assert!(lifecycle.set_foreground(false).is_err());
        }
        assert_eq!(totals(&database.path())[0], 1);
        assert_eq!(totals(&database.path())[6], 2);
    }
}

struct FullAnalyzer {
    analyzer: yomoku_analysis::Analyzer,
    trace: mpsc::Sender<Trace>,
    worker: ThreadId,
}

fn txt_event(bytes: &[u8], encoding: Option<TextEncoding>) -> Event {
    Event::ImportTxt(TxtImport {
        original_bytes: bytes.to_vec(),
        filename: "本𠮷.txt".into(),
        encoding,
    })
}

fn srt_event(bytes: &[u8], encoding: Option<TextEncoding>) -> Event {
    Event::ImportSrt(SrtImport {
        original_bytes: bytes.to_vec(),
        filename: "本𠮷.srt".into(),
        encoding,
    })
}

fn file_totals(path: &PathBuf) -> Vec<i64> {
    let connection = Connection::open(path).unwrap();
    let mut counts = totals(path);
    counts.push(
        connection
            .query_row("SELECT COUNT(*) FROM imported_files", [], |row| row.get(0))
            .unwrap(),
    );
    counts
}

#[test]
fn txt_cancel_bypasses_blocked_analysis_retains_completed_files_and_retries_fresh() {
    cancel_bypasses_blocked_analysis_retains_completed_files_and_retries_fresh(txt_event);
}

#[test]
fn srt_cancel_bypasses_blocked_analysis_retains_completed_files_and_retries_fresh() {
    cancel_bypasses_blocked_analysis_retains_completed_files_and_retries_fresh(srt_event);
}

fn cancel_bypasses_blocked_analysis_retains_completed_files_and_retries_fresh(
    import_event: fn(&[u8], Option<TextEncoding>) -> Event,
) {
    let database = Database::new();
    let first = Storage::open(database.path())
        .unwrap()
        .import_analyzed_txt(
            "猫".as_bytes(),
            "completed.txt",
            Some(yomoku_import::txt::TextEncoding::Utf8),
            100,
            |chapter| Ok::<_, DocumentError>(fixture(chapter)),
        )
        .unwrap();
    let first = match first {
        yomoku_storage::TxtImportOutcome::Created(value) => value.document,
        _ => unreachable!(),
    };
    let mut observer = Storage::open(database.path()).unwrap();
    let cat = observer
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    observer
        .set_vocabulary_status(cat.id, VocabularyStatus::Learning, 200)
        .unwrap();
    let before = file_totals(&database.path());
    let (release, gate) = mpsc::channel();
    let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    lifecycle.cancel_file_import().unwrap(); // Idle cancellation cannot poison the next attempt.
    let (done, result) = mpsc::channel();
    router.update(
        import_event("犬 犬".as_bytes(), Some(TextEncoding::Utf8)),
        move |_| {
            done.send(thread::current().id()).unwrap();
        },
    );
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Load(_)
    ));
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Analyze(_)
    ));
    // A rejected duplicate callback must not replace the accepted cancellation handle.
    assert!(router
        .update(import_event(b"rejected", None), |_| panic!(
            "busy event accepted"
        ))
        .is_empty());
    let lifecycle = thread::spawn(move || {
        lifecycle.cancel_file_import().unwrap();
        lifecycle.cancel_file_import().unwrap();
        lifecycle
    })
    .join()
    .unwrap();
    assert!(router.view().storage_busy);
    assert!(result.try_recv().is_err());
    assert_eq!(file_totals(&database.path()), before);
    lifecycle.set_foreground(false).unwrap();
    release.send(()).unwrap();
    assert_ne!(
        result.recv_timeout(TIMEOUT).unwrap(),
        thread::current().id()
    );
    let cancelled = router.view();
    assert!(!cancelled.storage_busy);
    assert!(cancelled.document_error.is_none() && cancelled.import_summary.is_none());
    assert_eq!(cancelled.documents.len(), 1);
    dispatch(&router, Event::RefreshLibrary); // Also drains queued background release.
    assert_eq!(file_totals(&database.path()), before);
    assert_eq!(
        observer
            .load_document_reader_snapshot(first.document_id)
            .unwrap()
            .unwrap()
            .document,
        first.clone().into()
    );
    assert_eq!(
        observer
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Learning
    );
    lifecycle.cancel_file_import().unwrap();
    let retry = dispatch(
        &router,
        import_event("犬 犬".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert_eq!(retry.documents.len(), 2);
    let summary = retry.import_summary.unwrap();
    assert_eq!(summary.outcome, yomoku_core::app::ImportOutcome::Created);
    assert_eq!(
        i64::from(summary.document_id),
        i64::from(first.document_id) + 1
    );
    lifecycle.cancel_file_import().unwrap(); // A completed file always remains completed.
    assert_eq!(observer.list_documents().unwrap().len(), 2);
    assert_eq!(
        observer
            .vocabulary_entry(&Surface::new("犬").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
}

#[test]
fn txt_cancel_during_full_load_skips_analysis_and_does_not_poison_encoding_retry() {
    cancel_during_full_load_skips_analysis_and_does_not_poison_encoding_retry(
        txt_event,
        DocumentError::TxtEncodingRequired,
    );
}

#[test]
fn srt_cancel_during_full_load_skips_analysis_and_does_not_poison_encoding_retry() {
    cancel_during_full_load_skips_analysis_and_does_not_poison_encoding_retry(
        srt_event,
        DocumentError::SrtEncodingRequired,
    );
}

fn cancel_during_full_load_skips_analysis_and_does_not_poison_encoding_retry(
    import_event: fn(&[u8], Option<TextEncoding>) -> Event,
    encoding_required: DocumentError,
) {
    let database = Database::new();
    let (entered, ready) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let analyzed = Arc::new(AtomicUsize::new(0));
    let calls = analyzed.clone();
    let mut gate = Some(gate);
    let loader: AnalyzerLoader = Box::new(move || {
        if let Some(gate) = gate.take() {
            entered.send(()).unwrap();
            gate.recv_timeout(TIMEOUT).unwrap();
        }
        let calls = calls.clone();
        Ok(Box::new(move |chapter: &ChapterText| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(fixture(chapter))
        }))
    });
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let before = file_totals(&database.path());
    let (done, result) = mpsc::channel();
    router.update(import_event(b"waiting", None), move |_| {
        done.send(()).unwrap()
    });
    ready.recv_timeout(TIMEOUT).unwrap();
    lifecycle.cancel_file_import().unwrap();
    assert!(router.view().storage_busy);
    release.send(()).unwrap();
    result.recv_timeout(TIMEOUT).unwrap();
    assert!(!router.view().storage_busy);
    assert!(router.view().document_error.is_none() && router.view().import_summary.is_none());
    assert_eq!(analyzed.load(Ordering::SeqCst), 0);
    assert_eq!(file_totals(&database.path()), before);
    assert_eq!(
        dispatch(&router, import_event(&[0xC2, 0xA9], None)).document_error,
        Some(encoding_required)
    );
    lifecycle.cancel_file_import().unwrap();
    let retry = dispatch(
        &router,
        import_event(&[0xC2, 0xA9], Some(TextEncoding::Utf8)),
    );
    assert!(retry.document_error.is_none() && retry.import_summary.is_some());
    assert_eq!(analyzed.load(Ordering::SeqCst), 1);
}

#[test]
fn txt_cancel_remains_available_while_closed_router_drains_accepted_analysis() {
    cancel_remains_available_while_closed_router_drains_accepted_analysis(txt_event);
}

#[test]
fn srt_cancel_remains_available_while_closed_router_drains_accepted_analysis() {
    cancel_remains_available_while_closed_router_drains_accepted_analysis(srt_event);
}

fn cancel_remains_available_while_closed_router_drains_accepted_analysis(
    import_event: fn(&[u8], Option<TextEncoding>) -> Event,
) {
    let database = Database::new();
    let (release, gate) = mpsc::channel();
    let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let before = file_totals(&database.path());
    router.update(import_event(b"draining", None), |_| {});
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Load(_)
    ));
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Analyze(_)
    ));
    drop(router);
    assert!(lifecycle.set_foreground(false).is_err());
    lifecycle.cancel_file_import().unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Drop(_)
    ));
    assert_eq!(file_totals(&database.path()), before);
    assert!(Storage::open(database.path())
        .unwrap()
        .list_documents()
        .unwrap()
        .is_empty());
}

#[test]
fn txt_cancel_reaches_queued_attempt_before_fifo_can_start_it() {
    cancel_reaches_queued_attempt_before_fifo_can_start_it(txt_event);
}

#[test]
fn srt_cancel_reaches_queued_attempt_before_fifo_can_start_it() {
    cancel_reaches_queued_attempt_before_fifo_can_start_it(srt_event);
}

fn cancel_reaches_queued_attempt_before_fifo_can_start_it(
    import_event: fn(&[u8], Option<TextEncoding>) -> Event,
) {
    struct GatedDrop {
        entered: mpsc::Sender<()>,
        gate: Option<mpsc::Receiver<()>>,
    }
    impl PasteAnalyzer for GatedDrop {
        fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
            Ok(fixture(chapter))
        }
    }
    impl Drop for GatedDrop {
        fn drop(&mut self) {
            if let Some(gate) = self.gate.take() {
                self.entered.send(()).unwrap();
                gate.recv_timeout(TIMEOUT).unwrap();
            }
        }
    }
    let database = Database::new();
    let (entered, ready) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let mut gate = Some(gate);
    let loads = Arc::new(AtomicUsize::new(0));
    let calls = loads.clone();
    let loader: AnalyzerLoader = Box::new(move || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(GatedDrop {
            entered: entered.clone(),
            gate: gate.take(),
        }))
    });
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, import_event(b"completed", None));
    let before = file_totals(&database.path());
    lifecycle.set_foreground(false).unwrap();
    ready.recv_timeout(TIMEOUT).unwrap(); // FIFO blocked releasing its previous analyzer.
    let (done, result) = mpsc::channel();
    router.update(import_event(b"queued", None), move |_| {
        done.send(()).unwrap()
    });
    lifecycle.cancel_file_import().unwrap();
    assert!(router.view().storage_busy);
    release.send(()).unwrap();
    result.recv_timeout(TIMEOUT).unwrap();
    assert!(!router.view().storage_busy);
    assert!(router.view().document_error.is_none() && router.view().import_summary.is_none());
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert_eq!(file_totals(&database.path()), before);
    assert!(dispatch(&router, import_event(b"queued", None))
        .import_summary
        .is_some());
    assert_eq!(loads.load(Ordering::SeqCst), 2);
}

#[test]
fn txt_typed_decoding_errors_skip_full_and_manual_choices_retry_without_partial_files() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    let before = file_totals(&database.path());
    for (bytes, encoding, error) in [
        (&[0xC2, 0xA9][..], None, DocumentError::TxtEncodingRequired),
        (
            &[0x82, 0xA0],
            Some(TextEncoding::Utf8),
            DocumentError::TxtInvalidUtf8,
        ),
        (
            &[0xFF],
            Some(TextEncoding::ShiftJis),
            DocumentError::TxtInvalidShiftJis,
        ),
        (
            &[0xFF, 0xFE, 0x41, 0],
            None,
            DocumentError::TxtUnsupportedEncoding,
        ),
        (&[0xFF], None, DocumentError::TxtUndecodable),
        (b" \r\n", None, DocumentError::EmptyTxt),
    ] {
        let rejected = dispatch(&router, txt_event(bytes, encoding));
        assert_eq!(rejected.document_error, Some(error));
        assert!(rejected.documents.is_empty() && rejected.import_summary.is_none());
        assert_eq!(file_totals(&database.path()), before);
    }
    let mut blank = match txt_event(b"valid", None) {
        Event::ImportTxt(input) => input,
        _ => unreachable!(),
    };
    blank.filename = " \r\n".into();
    assert_eq!(
        dispatch(&router, Event::ImportTxt(blank)).document_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(trace.try_recv().is_err());
    let selected = dispatch(
        &router,
        txt_event(&[0xC2, 0xA9], Some(TextEncoding::ShiftJis)),
    );
    assert_eq!(
        selected.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Created
    );
    let mut storage = Storage::open(database.path()).unwrap();
    let id = selected.documents[0].document_id;
    assert_eq!(storage.load_paste(id).unwrap().unwrap().chapter.text, "ﾂｩ");
    assert_eq!(
        storage.load_txt_encoding(id).unwrap().unwrap(),
        yomoku_import::txt::TextEncoding::ShiftJis
    );
    let unicode = dispatch(
        &router,
        txt_event("猫".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert!(unicode.document_error.is_none());
    let legacy = dispatch(&router, txt_event(&[0x82, 0xA0], None));
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
fn txt_duplicates_after_reopen_preserve_source_status_progress_and_skip_unavailable_full() {
    let database = Database::new();
    let bytes = "｜𠮷《よし》 猫 猫\r\nか\u{3099} English 。 📖 \0".as_bytes();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(
        Event::SelectDestination(Destination::Vocabulary),
        |_| unreachable!(),
    );
    let created = dispatch(&router, txt_event(bytes, Some(TextEncoding::Utf8)));
    assert_eq!(created.destination, Destination::Library);
    assert_eq!(created.content_route, ContentRoute::Main);
    assert!(created.reader.is_none());
    let id = created.import_summary.as_ref().unwrap().document_id;
    assert_eq!(created.documents[0].title, "本𠮷.txt");
    let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
        Trace::Load(id) => id,
        other => panic!("{other:?}"),
    };
    assert_ne!(worker, thread::current().id());
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    let opened = dispatch(&router, Event::OpenReader(id));
    assert!(matches!(
        opened.reader_analysis,
        Some(ReaderAnalysis::Available(_))
    ));
    let position = yomoku_core::pagination::ReadingPosition {
        offset: TextOffset::try_from(5).unwrap(),
        ..opened.reader.unwrap().position
    };
    let saved = dispatch(&router, Event::SaveReadingPosition(position));
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = Surface::new("猫").unwrap();
    let cat_entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(cat_entry.id, VocabularyStatus::Known, 123)
        .unwrap()
        .unwrap();
    let source = storage.load_txt_encoding(id).unwrap().unwrap();
    assert_eq!(source, yomoku_import::txt::TextEncoding::Utf8);
    let before = file_totals(&database.path());
    lifecycle.set_foreground(false).unwrap();
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
    let mut duplicate = match txt_event(bytes, Some(TextEncoding::ShiftJis)) {
        Event::ImportTxt(input) => input,
        _ => unreachable!(),
    };
    duplicate.filename.clear(); // existing identity takes priority over all new-file metadata
    let existing = dispatch(&router, Event::ImportTxt(duplicate.clone()));
    assert_eq!(
        existing.import_summary.as_ref().unwrap().outcome,
        yomoku_core::app::ImportOutcome::Existing
    );
    assert_eq!(existing.documents, saved.documents);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let existing = dispatch(&router, Event::ImportTxt(duplicate));
    assert_eq!(existing.import_summary.as_ref().unwrap().document_id, id);
    assert_eq!(existing.documents, saved.documents);
    assert_eq!(file_totals(&database.path()), before);
    assert_eq!(
        storage.load_paste(id).unwrap().unwrap().reading_position,
        Some(position)
    );
    assert_eq!(storage.load_txt_encoding(id).unwrap(), Some(source));
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), Some(known));
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn txt_loader_analysis_and_invalid_output_failures_preserve_completed_files_and_retry() {
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
            txt_event("猫 𠮷".as_bytes(), Some(TextEncoding::Utf8)),
        );
        assert_eq!(
            failed.document_error,
            Some(if mode == 2 {
                DocumentError::InvalidDocument
            } else {
                DocumentError::Unavailable
            })
        );
        assert_eq!(file_totals(&database.path()), vec![0; 13]);
    }
    failure.store(0, Ordering::Relaxed);
    let created = dispatch(
        &router,
        txt_event("猫 𠮷".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert!(created.document_error.is_none());
    let before = file_totals(&database.path());
    failure.store(1, Ordering::Relaxed);
    let rejected = dispatch(
        &router,
        txt_event("犬".as_bytes(), Some(TextEncoding::Utf8)),
    );
    assert_eq!(rejected.documents, created.documents);
    assert!(rejected.import_summary.is_none());
    assert_eq!(file_totals(&database.path()), before);
    failure.store(0, Ordering::Relaxed);
    assert_eq!(
        dispatch(
            &router,
            txt_event("犬".as_bytes(), Some(TextEncoding::Utf8))
        )
        .documents
        .len(),
        2
    );
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn txt_each_write_and_commit_failure_is_atomic_through_worker() {
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
        let created = dispatch(&router, txt_event(b"English", None));
        let before = file_totals(&database.path());
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
        let failed = dispatch(&router, txt_event(bytes, Some(TextEncoding::Utf8)));
        assert_eq!(
            failed.document_error,
            Some(DocumentError::Unavailable),
            "{table}"
        );
        assert_eq!(failed.documents, created.documents);
        assert_eq!(file_totals(&database.path()), before, "{table}");
        connection
            .execute_batch("DROP TRIGGER reject_import")
            .unwrap();
        assert_eq!(
            dispatch(&router, txt_event(bytes, Some(TextEncoding::Utf8)))
                .documents
                .len(),
            2
        );
        finish_worker(router, lifecycle, &trace);
    }
}

#[test]
fn txt_background_and_close_drain_atomic_import_and_reject_duplicate_callbacks() {
    background_and_close_drain_atomic_import_and_reject_duplicate_callbacks(txt_event);
}

#[test]
fn srt_background_and_close_drain_atomic_import_and_reject_duplicate_callbacks() {
    background_and_close_drain_atomic_import_and_reject_duplicate_callbacks(srt_event);
}

fn background_and_close_drain_atomic_import_and_reject_duplicate_callbacks(
    import_event: fn(&[u8], Option<TextEncoding>) -> Event,
) {
    for close in [false, true] {
        let database = Database::new();
        let (release, gate) = mpsc::channel();
        let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let event = import_event("猫 𠮷".as_bytes(), Some(TextEncoding::Utf8));
        let (sender, receiver) = mpsc::channel();
        router.update(event.clone(), move |_| {
            let _ = sender.send(());
        });
        let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
            Trace::Load(id) => id,
            other => panic!("{other:?}"),
        };
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
        assert_eq!(file_totals(&database.path()), vec![0; 13]);
        assert!(router.update(event, |_| unreachable!()).is_empty());
        lifecycle.set_foreground(false).unwrap();
        let router = if close {
            drop(router);
            None
        } else {
            Some(router)
        };
        release.send(()).unwrap();
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
        if let Some(router) = router {
            receiver.recv_timeout(TIMEOUT).unwrap();
            assert_eq!(router.view().documents.len(), 1);
            assert!(router.view().reader.is_none());
            finish_worker(router, lifecycle, &trace);
        } else {
            assert!(drain_trace(&trace).is_empty());
            assert!(lifecycle.set_foreground(true).is_err());
        }
        assert_eq!(file_totals(&database.path())[0], 1);
        assert_eq!(file_totals(&database.path())[12], 1);
    }
}
impl PasteAnalyzer for FullAnalyzer {
    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        self.analyzer
            .analyze_components(chapter, mode, profile)
            .map_err(|_| DocumentError::Unavailable)
    }
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        assert_eq!(thread::current().id(), self.worker);
        self.trace.send(Trace::Analyze(self.worker)).unwrap();
        self.analyzer
            .analyze(chapter)
            .map_err(|_| DocumentError::Unavailable)
    }
}
impl Drop for FullAnalyzer {
    fn drop(&mut self) {
        assert_eq!(thread::current().id(), self.worker);
        let _ = self.trace.send(Trace::Drop(self.worker));
    }
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn real_full_worker_import_and_lazy_reopen_preserve_mode_c_unicode_and_vocabulary() {
    // Real debug Full initialization can exceed the data-free harness deadline.
    let dispatch = |router: &EffectRouter, event| {
        dispatch_with_timeout(router, event, Duration::from_secs(60))
    };
    let dictionary = std::env::var_os("YOMOKU_SUDACHI_DICTIONARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.local/sudachi/full-20261015/system_full.dic")
        });
    let mapped = match std::env::var("YOMOKU_SUDACHI_STORAGE").as_deref() {
        Ok("mapped") => true,
        Ok("owned") | Err(_) => false,
        Ok(other) => panic!("unknown YOMOKU_SUDACHI_STORAGE: {other}"),
    };
    let (sender, trace) = mpsc::channel();
    let loader = Box::new(move || {
        let worker = thread::current().id();
        sender.send(Trace::Load(worker)).unwrap();
        let analyzer = if mapped {
            let file = std::fs::File::open(&dictionary).unwrap();
            // SAFETY: the explicit test runner keeps the prepared dictionary
            // immutable for the entire worker lifetime. No fixture rewrites it.
            unsafe { yomoku_analysis::Analyzer::open_full_mapped(&file) }
        } else {
            yomoku_analysis::Analyzer::open_full(&dictionary)
        }
        .unwrap();
        Ok(Box::new(FullAnalyzer {
            analyzer,
            trace: sender.clone(),
            worker,
        }) as Box<dyn PasteAnalyzer>)
    });
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = Surface::new("猫").unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&cat), 10)
        .unwrap();
    let cat_entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let familiar = storage
        .set_vocabulary_status(cat_entry.id, VocabularyStatus::Familiar, 20)
        .unwrap()
        .unwrap();
    let (router, lifecycle) = yomoku_runtime::effect_router(
        database.path(),
        loader,
        Box::new(|| {
            Ok(std::env::var_os("YOMOKU_WORD_DICTIONARY")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../.local/dictionaries/index-v1-20261025/dictionary.sqlite3")
                }))
        }),
    )
    .unwrap();
    dispatch(&router, Event::RefreshLibrary);
    assert!(trace.try_recv().is_err());
    let text = "国家公務員。猫と猫が食べた。\r\nｶﾞｯﾂﾎﾟｰｽﾞと𰻞 English 123 📖\0猫";
    let imported = dispatch(&router, Event::CreatePaste(text.into()));
    assert!(imported.document_error.is_none());
    assert!(imported.reader.is_none());
    let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
        Trace::Load(id) => id,
        other => panic!("{other:?}"),
    };
    assert_ne!(worker, thread::current().id());
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    let document = storage
        .load_paste(imported.documents[0].document_id)
        .unwrap()
        .unwrap();
    assert_eq!(document.chapter.text, text);
    let connection = Connection::open(database.path()).unwrap();
    let id: i64 = connection
        .query_row(
            "SELECT id FROM chapter_analyses WHERE chapter_id = ?1",
            [i64::from(document.chapter.chapter_id)],
            |row| row.get(0),
        )
        .unwrap();
    let analysis = storage
        .chapter_analysis(id.try_into().unwrap())
        .unwrap()
        .unwrap();
    analysis.validate(&document.chapter).unwrap();
    assert_eq!(analysis.mode, SplitMode::C);
    assert_eq!(
        analysis.profile.engine_revision,
        format!(
            "{}{}",
            yomoku_analysis::ENGINE_REVISION,
            yomoku_core::analysis::IMPORT_POLICY_SUFFIX
        )
    );
    assert_eq!(
        analysis.profile.dictionary_revision,
        format!(
            "{}{}{}",
            yomoku_analysis::DICTIONARY_REVISION,
            yomoku_core::analysis::WORD_DICTIONARY_PREFIX,
            yomoku_storage::dictionary::IMPORT_DICTIONARY_SHA256
        )
    );
    let surfaces: Vec<_> = analysis
        .tokens
        .iter()
        .map(|token| token.surface.as_str())
        .collect();
    assert_eq!(surfaces[0], "国家公務員");
    assert!(surfaces.iter().filter(|surface| **surface == "猫").count() >= 3);
    for token in &analysis.tokens {
        let entry = storage
            .vocabulary_entry(&Surface::new(&token.surface).unwrap())
            .unwrap();
        if token.is_japanese {
            let entry = entry.unwrap();
            if token.surface == "猫" {
                assert_eq!(entry, familiar);
            } else {
                assert_eq!(entry.status, VocabularyStatus::New);
                assert!(entry.registered_at_unix_ms >= 20);
            }
        } else {
            assert!(entry.is_none());
        }
    }
    let before = totals(&database.path());
    assert_eq!(&before[9..], &[0, 0, 0]);
    let duplicate = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(duplicate.documents, imported.documents);
    assert!(trace.try_recv().is_err());
    lifecycle.set_foreground(false).unwrap();
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
    // FIFO barrier: the old analyzer and its mapping have actually dropped.
    dispatch(&router, Event::RefreshLibrary);
    lifecycle.set_foreground(true).unwrap();
    assert_eq!(
        dispatch(&router, Event::CreatePaste(text.into())).documents,
        imported.documents
    );
    assert!(trace.try_recv().is_err());
    dispatch(&router, Event::CreatePaste("𰻞と猫。".into()));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Load(worker));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    assert_eq!(storage.list_documents().unwrap().len(), 2);
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), Some(familiar));
    assert_eq!(before[4], 1);
    finish_worker(router, lifecycle, &trace);
}

#[path = "imports/srt.rs"]
mod srt;

#[path = "imports/file_references.rs"]
mod file_references;

#[path = "imports/epub.rs"]
mod epub;

#[path = "imports/epub_assets.rs"]
mod epub_assets;

#[path = "../../yomoku-import/tests/support/epub.rs"]
mod epub_fixture;
