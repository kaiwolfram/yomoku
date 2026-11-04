//! Exercise the activation candidate through serialized shell events/views, not
//! direct Core updates. Native requests/results still stay inside the FIFO.
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
            .create_analyzed_paste(
                &PasteText::new("東京 東京 ヌヌ𠮷か\u{3099}\0".into()).unwrap(),
                100,
                analysis,
            )
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
        let (bridge, lifecycle) = YomokuBridge::with_effective_reader(
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
    let missing = Storage::open(f.path())
        .unwrap()
        .create_paste(&PasteText::new("未解析".into()).unwrap())
        .unwrap();
    let h = f.bridge();
    let loaded = h.load(missing.document_id);
    let page = loaded.effective_reader_tokens.as_ref().unwrap();
    assert!(page.tokens.is_empty());
    assert!(loaded.reader_tokens.is_none());
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
    let page = loaded.reader.as_ref().unwrap();
    let before = std::fs::read(f.path()).unwrap();
    for event in [
        Event::CompleteReaderPage {
            document_id: f.id,
            page: page.anchor,
            position: page.position,
            completed_at_unix_ms: 500,
        },
        Event::LoadReaderWord(yomoku_core::reader::ReaderWordSelection {
            key: loaded
                .effective_reader_tokens
                .as_ref()
                .unwrap()
                .identity
                .base,
            token_index: 0,
        }),
    ] {
        h.send(event);
        assert!(!h.view().storage_busy);
        assert!(h.callbacks.try_recv().is_err());
    }
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    assert_eq!(f.count("reading_completions"), 0);
}
