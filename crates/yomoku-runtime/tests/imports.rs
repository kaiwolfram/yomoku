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
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    storage::DocumentError,
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_runtime::{
    effect_router_with_analyzed_imports, AnalyzerLifecycle, AnalyzerLoader, EffectRouter,
    PasteAnalyzer, ShellEffect,
};
use yomoku_storage::Storage;

const TIMEOUT: Duration = Duration::from_secs(10);

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
        receiver.recv_timeout(TIMEOUT).unwrap(),
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
                analysis.mode = SplitMode::C;
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
        mode: SplitMode::B,
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
    // A retained lifecycle handle cannot keep the factory or worker alive.
    assert!(lifecycle.set_foreground(true).is_err());
    drain_trace(trace);
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
fn library_settings_and_invalid_pastes_never_load_analyzer() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(path, loader).unwrap();
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    let text = "猫 猫\r\n𠮷 か\u{3099} English 。 📖 \0";
    let view = dispatch(&router, Event::CreatePaste(text.into()));
    assert!(view.document_error.is_none());
    assert_eq!(view.content_route, ContentRoute::Main);
    assert!(view.reader.is_none() && view.loaded_document.is_none());
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    // Existing result alone must populate a fresh Library without a preceding list.
    let duplicate = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(duplicate.documents, saved.documents);
    assert!(matches!(
        trace.recv_timeout(TIMEOUT).unwrap(),
        Trace::Load(_)
    ));
    assert!(trace.try_recv().is_err());
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
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
    assert!(lifecycle.set_foreground(false).is_err());
    let events = drain_trace(&trace);
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
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
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
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Load(worker));
    assert!(trace.try_recv().is_err()); // duplicate did not tokenize
    drop(router);
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Drop(worker));
    assert!(trace.recv_timeout(TIMEOUT).is_err());
    assert!(lifecycle.set_foreground(false).is_err());
}

#[test]
fn background_and_close_wait_for_accepted_atomic_work_without_blocking_dispatch() {
    for close in [false, true] {
        let database = Database::new();
        let (release, gate) = mpsc::channel();
        let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
        let (router, lifecycle) =
            effect_router_with_analyzed_imports(database.path(), loader).unwrap();
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
            assert!(trace.recv_timeout(TIMEOUT).is_err());
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
impl PasteAnalyzer for FullAnalyzer {
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
fn real_full_worker_import_and_lazy_reopen_preserve_mode_b_unicode_and_vocabulary() {
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
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    dispatch(&router, Event::RefreshLibrary);
    assert!(trace.try_recv().is_err());
    let text = "国家公務員。猫と猫が食べた。\r\nｶﾞｯﾂﾎﾟｰｽﾞと𰻞 English 123 📖\0猫";
    let imported = dispatch(&router, Event::CreatePaste(text.into()));
    assert!(imported.document_error.is_none());
    assert!(imported.reader.is_none() && imported.loaded_document.is_none());
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
    assert_eq!(analysis.mode, SplitMode::B);
    assert_eq!(
        analysis.profile.engine_revision,
        yomoku_analysis::ENGINE_REVISION
    );
    assert_eq!(
        analysis.profile.dictionary_revision,
        yomoku_analysis::DICTIONARY_REVISION
    );
    let surfaces: Vec<_> = analysis
        .tokens
        .iter()
        .map(|token| token.surface.as_str())
        .collect();
    assert_eq!(&surfaces[..2], &["国家", "公務員"]);
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
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Load(worker));
    assert!(trace.try_recv().is_err());
    dispatch(&router, Event::CreatePaste("𰻞と猫。".into()));
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    assert_eq!(storage.list_documents().unwrap().len(), 2);
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), Some(familiar));
    assert_eq!(before[4], 1);
    finish_worker(router, lifecycle, &trace);
}
