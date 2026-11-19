#[path = "../../../tests/support/storage.rs"]
mod storage_fixture;
// Exercise the activation candidate through serialized shell events/views, not
// direct Core updates. Native requests/results still stay inside the FIFO.
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use rusqlite::Connection;
use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::{Event, ViewModel},
    document::{ChapterText, DocumentId, PasteText, TextAnchor, TextOffset, TextRange},
    reader::ReaderTokenKey,
    storage::DocumentError,
    vocabulary::Surface,
};
use yomoku_ffi::{FfiError, YomokuBridge};
use yomoku_runtime::{AnalyzerLifecycle, AnalyzerLoader, ShellEffectFfi};
use yomoku_storage::Storage;

const TIMEOUT: Duration = Duration::from_secs(10);

struct Fixture {
    directory: PathBuf,
    id: DocumentId,
}

fn analysis(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut cursor = 0;
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "ffi@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::C,
        tokens: chapter
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
                    is_japanese: true,
                    reading: Some(format!("読み{surface}")),
                    dictionary_form: Some(Surface::new(surface).unwrap()),
                }
            })
            .collect(),
    })
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "yomoku-effective-ffi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let id = Storage::open(directory.join("user.sqlite3"))
            .unwrap()
            .import_analyzed_paste(
                &PasteText::new("東京 東京 ヌヌ𠮷か\u{3099}\0".into()).unwrap(),
                100,
                analysis,
            )
            .map(|outcome| match outcome {
                yomoku_storage::PasteImportOutcome::Created(value) => value,
                _ => panic!("expected fresh import"),
            })
            .unwrap()
            .document
            .document_id;
        let mut storage = Storage::open(directory.join("user.sqlite3")).unwrap();
        let document = storage
            .load_document_reader_snapshot(id)
            .unwrap()
            .unwrap()
            .document;
        for mode in [SplitMode::A, SplitMode::B] {
            let mut component = analysis(&document.chapters[0]).unwrap();
            component.mode = mode;
            storage
                .store_chapter_analysis(document.chapters[0].chapter_id, &component)
                .unwrap();
        }
        let index = Connection::open(directory.join("words.sqlite3")).unwrap();
        index.execute_batch("PRAGMA application_id=0x594f4449; PRAGMA user_version=1;
            CREATE TABLE entries(source TEXT,entry_id INTEGER,payload TEXT,PRIMARY KEY(source,entry_id)) WITHOUT ROWID;
            CREATE TABLE forms(surface TEXT COLLATE BINARY,source TEXT,entry_id INTEGER,PRIMARY KEY(surface,source,entry_id)) WITHOUT ROWID;").unwrap();
        index.execute("INSERT INTO entries VALUES ('JMdict',1,?1)", [r#"{"kanji":[{"text":"東京"}],"readings":[{"text":"とうきょう"}],"senses":[{"glosses":[{"text":"Tokyo","lang":"eng"}]}]}"#]).unwrap();
        index
            .execute("INSERT INTO forms VALUES ('東京','JMdict',1)", [])
            .unwrap();
        Self { directory, id }
    }

    fn path(&self) -> PathBuf {
        self.directory.join("user.sqlite3")
    }

    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }

    fn bridge(&self) -> Harness {
        self.bridge_with_loader(Box::new(|| {
            panic!("stored effective reads/writes must not load Sudachi")
        }))
    }

    fn bridge_with_loader(&self, loader: AnalyzerLoader) -> Harness {
        let (tx, callbacks) = mpsc::channel();
        let index = self.directory.join("words.sqlite3");
        let (bridge, lifecycle) = YomokuBridge::new(
            self.path(),
            loader,
            Box::new(move || Ok(index.clone())),
            move |result| tx.send((thread::current().id(), result)).unwrap(),
        )
        .unwrap();
        Harness {
            bridge,
            callbacks,
            _lifecycle: lifecycle,
        }
    }

    fn count(&self, table: &str) -> i64 {
        self.sql()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct Harness {
    bridge: YomokuBridge,
    callbacks: mpsc::Receiver<(thread::ThreadId, Result<Vec<u8>, FfiError>)>,
    _lifecycle: AnalyzerLifecycle,
}

fn renders(bytes: &[u8]) {
    let requests: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(bytes).unwrap();
    assert!(!requests.is_empty());
    assert!(requests
        .iter()
        .all(|request| matches!(request.effect, ShellEffectFfi::Render(_))));
}

impl Harness {
    fn view(&self) -> ViewModel {
        BincodeFfiFormat::deserialize(&self.bridge.view().unwrap()).unwrap()
    }

    fn send(&self, event: Event) -> Vec<u8> {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        self.bridge.update(&bytes).unwrap()
    }

    fn dispatch(&self, event: Event) -> ViewModel {
        self.dispatch_n(event, 1)
    }

    fn dispatch_n(&self, event: Event, callbacks: usize) -> ViewModel {
        renders(&self.send(event));
        for _ in 0..callbacks {
            let (worker, result) = self.callbacks.recv_timeout(TIMEOUT).unwrap();
            assert_ne!(worker, thread::current().id());
            renders(&result.unwrap());
        }
        let view = self.view();
        assert!(!view.storage_busy);
        assert!(self.callbacks.try_recv().is_err());
        view
    }

    fn load(&self, id: DocumentId) -> ViewModel {
        let opened = self.dispatch(Event::OpenReader(id));
        self.dispatch(Event::LoadReaderTokens(ReaderTokenKey {
            document_id: id,
            page: opened.reader.unwrap().anchor,
            analysis: opened.reader_analysis.unwrap(),
        }))
    }
}

#[test]
fn serialized_missing_analysis_and_legacy_commands_fail_closed() {
    let f = Fixture::new();
    let missing = storage_fixture::unanalyzed(
        &mut Storage::open(f.path()).unwrap(),
        f.path(),
        &PasteText::new("未解析".into()).unwrap(),
    );
    let h = f.bridge();
    let loaded = h.load(missing.document_id);
    let page = loaded.effective_reader_tokens.as_ref().unwrap();
    assert!(page.tokens.is_empty());
    let before = std::fs::read(f.path()).unwrap();
    h.send(Event::CompleteEffectiveReaderPage {
        identity: page.identity,
        position: loaded.reader.as_ref().unwrap().position,
        completed_at_unix_ms: 500,
    });
    assert_eq!(
        h.view().document_error,
        Some(DocumentError::MissingAnalysis)
    );
    assert!(h.callbacks.try_recv().is_err());
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    let loaded = h.load(f.id);
    let _page = loaded.reader.as_ref().unwrap();
    let before = std::fs::read(f.path()).unwrap();

    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    assert_eq!(f.count("reading_completions"), 0);
}

#[test]
#[ignore = "requires pinned Sudachi Full; run tools/test-sudachi.sh"]
fn native_previous_profile_book_lazily_loads_components_without_changing_boundaries() {
    use std::{fs::File, os::fd::IntoRawFd};
    use yomoku_analysis::Analyzer;
    use yomoku_core::{
        analysis::{LEGACY_IMPORT_POLICY_SUFFIX, WORD_DICTIONARY_PREFIX},
        effective_session::EffectiveReaderSelection,
        vocabulary::VocabularyStatus,
    };
    use yomoku_ffi::native::{DictionaryProvider, EffectsListener, NativeBridge};
    use yomoku_storage::dictionary::IMPORT_DICTIONARY_SHA256;

    struct Provider(PathBuf, PathBuf);
    impl DictionaryProvider for Provider {
        fn open_verified_dictionary(&self) -> i64 {
            i64::from(File::open(&self.0).unwrap().into_raw_fd()) + 1
        }
        fn verified_word_dictionary_path(&self) -> String {
            self.1.to_str().unwrap().into()
        }
    }
    struct Listener(mpsc::Sender<Result<Vec<u8>, FfiError>>);
    impl EffectsListener for Listener {
        fn on_effects(&self, bytes: Vec<u8>) {
            self.0.send(Ok(bytes)).unwrap();
        }
        fn on_error(&self, error: FfiError) {
            self.0.send(Err(error)).unwrap();
        }
    }
    let f = Fixture::new();
    let full = std::env::var_os("YOMOKU_SUDACHI_DICTIONARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.local/sudachi/full-20261015/system_full.dic")
        });
    let engine = Analyzer::open_full(&full).unwrap();
    let mut storage = Storage::open(f.path()).unwrap();
    let yomoku_storage::PasteImportOutcome::Created(imported) = storage
        .import_analyzed_paste(
            &PasteText::new("その日。食べました。".into()).unwrap(),
            100,
            |chapter| {
                // These are the saved v1 boundaries. The v2 matcher would join その日.
                let mut original = engine.analyze(chapter)?;
                assert_eq!(
                    original
                        .tokens
                        .iter()
                        .map(|t| t.surface.as_str())
                        .collect::<Vec<_>>(),
                    ["その", "日", "。", "食べました", "。"]
                );
                original
                    .profile
                    .engine_revision
                    .push_str(LEGACY_IMPORT_POLICY_SUFFIX);
                original
                    .profile
                    .dictionary_revision
                    .push_str(WORD_DICTIONARY_PREFIX);
                original
                    .profile
                    .dictionary_revision
                    .push_str(IMPORT_DICTIONARY_SHA256);
                Ok::<_, yomoku_analysis::AnalysisError>(original)
            },
        )
        .unwrap()
    else {
        panic!("expected new fixture")
    };
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    let surface = Surface::new("食べました").unwrap();
    let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Familiar, 200)
        .unwrap();
    for mode in [SplitMode::A, SplitMode::B] {
        assert!(!storage
            .has_component_analysis(
                imported.document.chapter.chapter_id,
                &original.profile,
                mode
            )
            .unwrap());
    }
    drop(storage);
    drop(engine);
    let words = f.directory.join("words.sqlite3");
    let index = Connection::open(&words).unwrap();
    index.execute("INSERT INTO entries VALUES ('JMdict',2,?1)", [r#"{"kanji":[{"text":"食べる"}],"readings":[{"text":"たべる"}],"senses":[{"glosses":[{"text":"eat","lang":"eng"}]}]}"#]).unwrap();
    index
        .execute("INSERT INTO forms VALUES ('食べる','JMdict',2)", [])
        .unwrap();
    drop(index);
    let (tx, rx) = mpsc::channel();
    let bridge = NativeBridge::new(
        f.path().to_str().unwrap().into(),
        Box::new(Listener(tx)),
        Box::new(Provider(full, words)),
    )
    .unwrap();
    let dispatch = |event: Event| -> ViewModel {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        renders(&bridge.update(&bytes).unwrap());
        renders(&rx.recv_timeout(Duration::from_secs(60)).unwrap().unwrap());
        let view: ViewModel = BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap();
        assert!(!view.storage_busy);
        assert_eq!(view.document_error, None);
        view
    };
    let opened = dispatch(Event::OpenReader(imported.document.document_id));
    let loaded = dispatch(Event::LoadReaderTokens(ReaderTokenKey {
        document_id: imported.document.document_id,
        page: opened.reader.unwrap().anchor,
        analysis: opened.reader_analysis.unwrap(),
    }));
    let page = loaded.effective_reader_tokens.unwrap();
    assert_eq!(
        page.tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["その", "日", "食べました"]
    );
    let selected = &page.tokens[2];
    assert_eq!(
        selected.vocabulary.unwrap().status,
        VocabularyStatus::Familiar
    );
    let view = dispatch(Event::LoadEffectiveReaderWord(EffectiveReaderSelection {
        page: page.identity,
        segment_index: selected.segment_index,
    }));
    let details = view.effective_reader_word.unwrap().details.unwrap();
    assert_eq!(details.token.surface, "食べました");
    assert_eq!(
        details.candidates.dictionary_form.as_deref(),
        Some("食べる")
    );
    assert!(!details.candidates.words.is_empty());
    assert!(details
        .lookup
        .parts
        .iter()
        .any(|part| part.surface == "食べ" && part.dictionary_form.as_deref() == Some("食べる")));
    let mut storage = Storage::open(f.path()).unwrap();
    assert_eq!(
        storage
            .chapter_analysis(imported.analysis_id)
            .unwrap()
            .unwrap(),
        original
    );
    for mode in [SplitMode::A, SplitMode::B] {
        assert!(storage
            .has_component_analysis(
                imported.document.chapter.chapter_id,
                &original.profile,
                mode
            )
            .unwrap());
    }
    assert!(storage
        .vocabulary_entry(&Surface::new("その日").unwrap())
        .unwrap()
        .is_none());
    assert_eq!(
        storage.vocabulary_entry(&surface).unwrap().unwrap().status,
        VocabularyStatus::Familiar
    );
}
