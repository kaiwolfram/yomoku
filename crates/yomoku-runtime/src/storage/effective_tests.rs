#[path = "../../../../tests/support/storage.rs"]
mod storage_fixture;
// Exercise the real middleware/worker, including completion retries.
use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use crux_core::{
    middleware::{HandleEffectLayer, Layer},
    render, App, Command, Core,
};
use rusqlite::Connection;
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::Effect,
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveReaderWordDetails,
    effective_session::{
        EffectiveReaderProgress, EffectiveReaderRequest, EffectiveReaderSelection,
        EffectiveReaderSession, EffectiveSelectionCommand,
    },
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::PageLength,
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_storage::{AnalyzedPaste, Storage};

use super::StorageEffects;

mod app;
mod requests;

const TIMEOUT: Duration = Duration::from_secs(10);
type Router = HandleEffectLayer<Core<Probe>, StorageEffects>;

#[derive(Default)]
struct Probe;
enum Event {
    Request(StorageOperation),
    ReaderRequest(EffectiveReaderRequest),
    Finished(StorageResult),
}
impl App for Probe {
    type Event = Event;
    type Model = Vec<StorageResult>;
    type ViewModel = Vec<StorageResult>;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Self::Model) -> Command<Effect, Event> {
        match event {
            Event::ReaderRequest(request) => {
                let expected = request.clone();
                request.command(move |captured, result| {
                    assert_eq!(captured, expected);
                    Event::Finished(result)
                })
            }
            Event::Request(operation) => {
                Command::request_from_shell(operation).then_send(Event::Finished)
            }
            Event::Finished(output) => {
                model.push(output);
                render::render()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.clone()
    }
}

struct Fixture {
    directory: PathBuf,
    imported: AnalyzedPaste,
    base: ReaderTokenKey,
}

fn analysis(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut cursor = 0;
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "worker@1".into(),
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
                    is_japanese: !matches!(surface, "English" | "。"),
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
            "yomoku-effective-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let mut storage = Storage::open(directory.join("user.sqlite3")).unwrap();
        let imported = storage
            .import_analyzed_paste(
                &PasteText::new("食べた 食べた 猫 English 。 𠮷か\u{3099}\0".into()).unwrap(),
                100,
                analysis,
            )
            .map(|outcome| match outcome {
                yomoku_storage::PasteImportOutcome::Created(value) => value,
                _ => panic!("expected fresh import"),
            })
            .unwrap();
        for mode in [SplitMode::A, SplitMode::B] {
            let mut component = analysis(&imported.document.chapter).unwrap();
            component.mode = mode;
            storage
                .store_chapter_analysis(imported.document.chapter.chapter_id, &component)
                .unwrap();
        }
        let document = storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap()
            .unwrap()
            .document;
        let page = ReaderPage::open_document(&document, PageLength::default()).unwrap();
        let base = ReaderTokenKey {
            document_id: document.document_id,
            page: page.anchor,
            analysis: ReaderAnalysis::Available(imported.analysis_id),
        };
        let index = Connection::open(directory.join("words.sqlite3")).unwrap();
        index.execute_batch("PRAGMA application_id = 0x594f4449; PRAGMA user_version = 1;
            CREATE TABLE entries (source TEXT, entry_id INTEGER, payload TEXT, PRIMARY KEY(source, entry_id)) WITHOUT ROWID;
            CREATE TABLE forms (surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER, PRIMARY KEY(surface, source, entry_id)) WITHOUT ROWID;").unwrap();
        index.execute("INSERT INTO entries VALUES ('JMdict', 1, ?1)", [r#"{"kanji":[{"text":"猫"}],"readings":[{"text":"ねこ"}],"senses":[{"pos":["noun (common) (futsuumeishi)"],"glosses":[{"text":"cat","lang":"eng"}]}]}"#]).unwrap();
        index
            .execute("INSERT INTO forms VALUES ('猫', 'JMdict', 1)", [])
            .unwrap();
        Self {
            directory,
            imported,
            base,
        }
    }

    fn path(&self) -> PathBuf {
        self.directory.join("user.sqlite3")
    }
    fn index(&self) -> PathBuf {
        self.directory.join("words.sqlite3")
    }
    fn storage(&self) -> Storage {
        Storage::open(self.path()).unwrap()
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
    fn router(&self) -> Router {
        let index = self.index();
        let (effects, _) = StorageEffects::new(
            self.path(),
            Box::new(|| panic!("effective commands must never load Sudachi")),
            Box::new(move || Ok(index.clone())),
        )
        .unwrap();
        Core::<Probe>::new().handle_effects_using(effects)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn enqueue(router: &Router, operation: StorageOperation) -> mpsc::Receiver<thread::ThreadId> {
    enqueue_event(router, Event::Request(operation))
}

fn enqueue_event(router: &Router, event: Event) -> mpsc::Receiver<thread::ThreadId> {
    let (sender, receiver) = mpsc::channel();
    let effects = router.update(event, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, Effect::Render(_))));
        sender.send(thread::current().id()).unwrap();
    });
    assert!(effects.is_empty()); // storage consumed in Rust, no shell request
    receiver
}

fn wait(receiver: mpsc::Receiver<thread::ThreadId>) {
    assert_ne!(
        receiver.recv_timeout(TIMEOUT).unwrap(),
        thread::current().id()
    );
}

fn run(router: &Router, operation: Op) -> StorageResult {
    wait(enqueue(router, StorageOperation::Effective(operation)));
    router.view().pop().unwrap()
}

fn load(router: &Router, session: &mut EffectiveReaderSession, base: ReaderTokenKey) {
    let request = session.begin_load(base).unwrap();
    assert_eq!(
        finish_request(router, session, EffectiveReaderRequest::Tokens(request)),
        EffectiveReaderProgress::TokensLoaded
    );
}

fn finish_request(
    router: &Router,
    session: &mut EffectiveReaderSession,
    request: EffectiveReaderRequest,
) -> EffectiveReaderProgress {
    wait(enqueue_event(router, Event::ReaderRequest(request.clone())));
    session
        .finish_request(&request, router.view().pop().unwrap())
        .unwrap()
}

fn select(session: &EffectiveReaderSession, surface: &str) -> EffectiveSelectionCommand {
    let page = session.page().unwrap();
    let token = page
        .tokens
        .iter()
        .find(|token| token.surface == surface)
        .unwrap();
    session
        .freeze_selection(EffectiveReaderSelection {
            page: page.identity,
            segment_index: token.segment_index,
        })
        .unwrap()
}

fn word(router: &Router, command: EffectiveSelectionCommand) -> EffectiveReaderWordDetails {
    let selection = command.selection();
    let StorageResult::Effective(Output::WordLoaded(returned, details)) =
        run(router, Op::LoadWord(command))
    else {
        panic!("expected word")
    };
    assert_eq!(returned, selection);
    *details
}

#[test]
fn effective_worker_missing_foreign_and_deleted_pages_fail_without_analysis_or_registration() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    let legacy = storage_fixture::unanalyzed(
        &mut fixture.storage(),
        fixture.path(),
        &PasteText::new("猫".into()).unwrap(),
    );
    let base = ReaderTokenKey {
        document_id: legacy.document_id,
        page: TextAnchor {
            source_id: legacy.chapter.source_id,
            chapter_id: legacy.chapter.chapter_id,
            range: TextRange {
                start: TextOffset::START,
                end: TextOffset::at(&legacy.chapter.text, legacy.chapter.text.len()).unwrap(),
            },
        },
        analysis: ReaderAnalysis::Missing,
    };
    load(&router, &mut session, base);
    assert!(session.page().unwrap().tokens.is_empty());
    assert!(session
        .freeze_page(session.page().unwrap().identity)
        .is_none());
    for base in [
        ReaderTokenKey {
            analysis: ReaderAnalysis::Missing,
            ..fixture.base
        },
        ReaderTokenKey {
            analysis: fixture.base.analysis,
            ..base
        },
    ] {
        let request = session.begin_load(base).unwrap();
        assert_eq!(
            run(&router, Op::LoadTokens(request)),
            StorageResult::Failed(DocumentError::InvalidDocument)
        );
        assert!(session.fail(request));
    }
    load(&router, &mut session, fixture.base);
    let command = select(&session, "猫");
    fixture
        .sql()
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(fixture.base.document_id)],
        )
        .unwrap();
    assert_eq!(
        run(&router, Op::LoadWord(command)),
        StorageResult::Failed(DocumentError::NotFound)
    );
    let request = session.begin_load(fixture.base).unwrap();
    assert_eq!(
        run(&router, Op::LoadTokens(request)),
        StorageResult::Failed(DocumentError::NotFound)
    );
}

#[test]
fn effective_worker_reads_exact_analyzer_metadata_live_status_and_literal_custom_entries() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    let before = std::fs::read(fixture.path()).unwrap();
    load(&router, &mut session, fixture.base);
    let manual = select(&session, "食べた");
    let details = word(&router, manual.clone());
    assert_eq!(
        details.token.metadata.analysis_id,
        details.key.segmentation.original_analysis_id
    );
    assert!(details.token.vocabulary.is_some()); // reads never register override parts
    assert!(details.candidates.dictionary_form.is_some());
    assert_eq!(details.encountered, 0);
    let cat = select(&session, "猫");
    assert_eq!(word(&router, cat.clone()).candidates.words.len(), 1);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let id = word(&router, cat.clone()).token.vocabulary.unwrap().id;
    fixture
        .storage()
        .set_vocabulary_status(id, VocabularyStatus::Ignored, 200)
        .unwrap();
    assert_eq!(
        word(&router, cat.clone()).token.vocabulary.unwrap().status,
        VocabularyStatus::Ignored
    );

    let command = select(&session, "𠮷か\u{3099}\0");
    let missing = word(&router, command);
    assert!(missing.candidates.words.is_empty() && missing.candidates.names.is_empty());
    // Session invalidation retires callbacks while accepted commands keep their key.
    session.invalidate();
    assert_eq!(word(&router, manual).token.surface, "食べた");
}

#[test]
fn effective_worker_dictionary_failures_require_explicit_retry_and_reopen_the_index() {
    let fixture = Fixture::new();
    let attempts = Arc::new(AtomicUsize::new(0));
    let trace = Arc::clone(&attempts);
    let index = fixture.index();
    let calling_thread = thread::current().id();
    let (effects, _) = StorageEffects::new(
        fixture.path(),
        Box::new(|| panic!("no analyzer")),
        Box::new(move || {
            assert_ne!(thread::current().id(), calling_thread);
            if trace.fetch_add(1, Ordering::Relaxed) == 0 {
                Err(DocumentError::Unavailable)
            } else {
                Ok(index.clone())
            }
        }),
    )
    .unwrap();
    let router = Core::<Probe>::new().handle_effects_using(effects);
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    assert_eq!(attempts.load(Ordering::Relaxed), 0);
    let command = select(&session, "猫");
    assert_eq!(
        run(&router, Op::LoadWord(command.clone())),
        StorageResult::Failed(DocumentError::Unavailable)
    );
    assert_eq!(attempts.load(Ordering::Relaxed), 1);
    let expected = word(&router, command.clone());
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    let index = Connection::open(fixture.index()).unwrap();
    let payload: String = index
        .query_row("SELECT payload FROM entries", [], |r| r.get(0))
        .unwrap();
    index
        .execute("UPDATE entries SET payload = '{}'", [])
        .unwrap();
    assert_eq!(
        run(&router, Op::LoadWord(command.clone())),
        StorageResult::Failed(DocumentError::Unavailable)
    );
    index
        .execute("UPDATE entries SET payload = ?1", [payload])
        .unwrap();
    assert_eq!(word(&router, command), expected);
    assert_eq!(attempts.load(Ordering::Relaxed), 3);
}

mod vocabulary;
