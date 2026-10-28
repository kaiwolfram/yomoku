//! Exercise the real middleware/worker, including queued legacy time saves.
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
use yomoku_core::effective_session::{
    EffectiveCompletionProgress as CompletionProgress, EffectiveCompletionRequest,
};
use yomoku_core::{
    active_time::{PendingReadingTime, ReadingTimeUpdate},
    analysis::{
        AnalysisProfile, AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride,
        SplitMode,
    },
    app::Effect,
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{ChapterText, OverrideId, PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveReaderWordDetails,
    effective_session::{
        EffectiveReaderSelection, EffectiveReaderSession, EffectiveSelectionCommand,
        EffectiveWordRequest,
    },
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::PageLength,
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_storage::{AnalyzedPaste, Storage};

use super::StorageEffects;

mod analysis;

const TIMEOUT: Duration = Duration::from_secs(10);
type Router = HandleEffectLayer<Core<Probe>, StorageEffects>;

#[derive(Default)]
struct Probe;
enum Event {
    Request(StorageOperation),
    Finished(StorageResult),
}
impl App for Probe {
    type Event = Event;
    type Model = Vec<StorageResult>;
    type ViewModel = Vec<StorageResult>;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Self::Model) -> Command<Effect, Event> {
        match event {
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
    rule: OverrideId,
    base: ReaderTokenKey,
    intent: PageCompletionIntent,
}

fn analysis(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    let mut cursor = 0;
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "worker@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::B,
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
            .create_analyzed_paste(
                &PasteText::new("食べた 食べた 猫 English 。 𠮷か\u{3099}\0".into()).unwrap(),
                100,
                analysis,
            )
            .unwrap();
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
        let rule = storage
            .save_segmentation_override(&SegmentationOverride {
                scope: OverrideScope::AllOccurrences,
                anchor: TextAnchor {
                    range: TextRange {
                        start: TextOffset::START,
                        end: TextOffset::at(&imported.document.chapter.text, "食べた".len())
                            .unwrap(),
                    },
                    ..base.page
                },
                surface: Surface::new("食べた").unwrap(),
                parts: ["食べ", "た"].map(|s| Surface::new(s).unwrap()).into(),
            })
            .unwrap();
        let intent = PageCompletionIntent {
            document_id: document.document_id,
            page: page.anchor,
            page_length: PageLength::default(),
            departing_position: page.position,
            completed_at_unix_ms: 500,
            analysis_id: imported.analysis_id,
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
            rule,
            base,
            intent,
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
        let (effects, _) = StorageEffects::with_dictionaries(
            self.path(),
            Box::new(|| panic!("effective commands must never load Sudachi")),
            Box::new(move || Ok(index.clone())),
        )
        .unwrap();
        Core::<Probe>::new().handle_effects_using(effects)
    }

    fn override_router(&self) -> Router {
        let (effects, _) = StorageEffects::with_dictionaries(
            self.path(),
            Box::new(|| panic!("boundary commands must never load Sudachi")),
            Box::new(|| panic!("boundary commands must never load word dictionaries")),
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
    let (sender, receiver) = mpsc::channel();
    let effects = router.update(Event::Request(operation), move |effects| {
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
    let StorageResult::Effective(Output::TokensLoaded(returned, snapshot)) =
        run(router, Op::LoadTokens(request))
    else {
        panic!("expected tokens")
    };
    assert_eq!(returned, request);
    assert!(session.accept(returned, snapshot));
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

fn finish_word(
    router: &Router,
    session: &mut EffectiveReaderSession,
    request: &EffectiveWordRequest,
) {
    let result = match run(router, request.operation()) {
        StorageResult::Effective(result) => Ok(result),
        StorageResult::Failed(error) => Err(error),
        result => panic!("unexpected word result: {result:?}"),
    };
    assert!(session.finish_word_request(request, result));
}

#[test]
fn effective_worker_retained_override_failure_reopen_lost_success_requires_explicit_reload() {
    use yomoku_core::{
        effective_overrides::EffectiveOverrideFailure as Failure,
        effective_session::{
            EffectiveOverrideAction as Shell, EffectiveOverrideProgress as Progress,
        },
    };
    let fixture = Fixture::new();
    let mut storage = fixture.storage();
    for mode in [SplitMode::A, SplitMode::C] {
        let mut alternate = analysis(&fixture.imported.document.chapter).unwrap();
        alternate.mode = mode;
        storage
            .store_chapter_analysis(fixture.base.page.chapter_id, &alternate)
            .unwrap();
    }
    let time = storage
        .load_pending_reading_time(fixture.base.document_id, fixture.intent.departing_position)
        .unwrap();
    storage
        .save_pending_reading_time(time, PendingReadingTime::from_millis(987).unwrap())
        .unwrap();
    let document = storage
        .load_document_reader_snapshot(fixture.base.document_id)
        .unwrap();
    drop(storage);
    let reading_state = || {
        fixture.sql().query_row(
        "SELECT (SELECT COUNT(*) FROM reading_completions), (SELECT COUNT(*) FROM reading_occurrences),
         (SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'),
         (SELECT SUM(milliseconds) FROM pending_reading_time)", [],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
    ).unwrap()
    };
    let before_reading = reading_state();
    let router = fixture.override_router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let identity = session.page().unwrap().identity;
    let anchor = session
        .page()
        .unwrap()
        .tokens
        .iter()
        .find(|token| token.surface == "𠮷か\u{3099}\0")
        .unwrap()
        .anchor;
    let request = session
        .handle_override_action(Shell::Open {
            page: identity,
            anchor,
        })
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let StorageResult::Effective(output @ Output::OverrideContextLoaded(_)) =
        run(&router, request.operation())
    else {
        panic!("context")
    };
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert_eq!(
        session.finish_override_request(&request, Ok(output)),
        Some(Progress::Loaded)
    );
    assert_eq!(
        session
            .override_context()
            .unwrap()
            .choices()
            .iter()
            .map(|choice| choice.mode)
            .collect::<Vec<_>>(),
        [SplitMode::A, SplitMode::B, SplitMode::C]
    );
    let prompt_identity = session.override_view().identity.unwrap();
    let request = session
        .handle_override_action(Shell::Save {
            identity: prompt_identity,
            scope: OverrideScope::ThisOccurrence,
            parts: vec!["𠮷".into(), "か\u{3099}\0".into()],
            registered_at_unix_ms: 600,
        })
        .unwrap();
    fixture.sql().execute_batch("CREATE TRIGGER fail_override BEFORE INSERT ON segmentation_overrides BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let StorageResult::Effective(output) = run(&router, request.operation()) else {
        panic!("failure")
    };
    assert_eq!(
        session.finish_override_request(&request, Ok(output)),
        Some(Progress::Failed(Failure::Storage(
            DocumentError::Unavailable
        )))
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert_eq!(reading_state(), before_reading);
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_override")
        .unwrap();
    drop(router);
    let router = fixture.override_router();
    let retry = session
        .handle_override_action(Shell::Retry {
            identity: prompt_identity,
        })
        .unwrap();
    assert_eq!(retry.operation(), request.operation());
    let StorageResult::Effective(Output::OverrideSaved { id, registered }) =
        run(&router, retry.operation())
    else {
        panic!("saved")
    };
    assert_eq!(registered, 2);
    // Lose the successful response, then delete one new vocabulary entry. The
    // retained before-image must become stale rather than recreating that row.
    session
        .finish_override_request(&retry, Err(DocumentError::Unavailable))
        .unwrap();
    fixture
        .sql()
        .execute("DELETE FROM vocabulary WHERE surface = ?1", ["𠮷"])
        .unwrap();
    let retry = session
        .handle_override_action(Shell::Retry {
            identity: prompt_identity,
        })
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let StorageResult::Effective(output) = run(&router, retry.operation()) else {
        panic!("stale")
    };
    assert_eq!(
        session.finish_override_request(&retry, Ok(output)),
        Some(Progress::Failed(Failure::StaleContext))
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert!(session.override_state().reload_required);
    assert!(session.retry_override_write().is_none());
    assert_eq!(reading_state(), before_reading);
    assert_eq!(
        fixture
            .storage()
            .load_document_reader_snapshot(fixture.base.document_id)
            .unwrap(),
        document
    );
    load(&router, &mut session, fixture.base);
    let read = session
        .handle_override_action(Shell::Open {
            page: session.page().unwrap().identity,
            anchor,
        })
        .unwrap();
    let StorageResult::Effective(output) = run(&router, read.operation()) else {
        panic!("reload")
    };
    session.finish_override_request(&read, Ok(output)).unwrap();
    assert_eq!(session.override_context().unwrap().rules()[0].id, id);
    let compact = session.override_view().prompt.unwrap();
    assert_eq!(compact.winning_rules[0].id, id);
    assert_eq!(compact.winning_rules[0].parts, vec!["𠮷", "か\u{3099}\0"]);
    let reverse = session
        .handle_override_action(Shell::Reverse {
            identity: compact.identity,
            id,
        })
        .unwrap();
    let StorageResult::Effective(output) = run(&router, reverse.operation()) else {
        panic!("reverse")
    };
    assert_eq!(
        session.finish_override_request(&reverse, Ok(output)),
        Some(Progress::Reversed(id))
    );
    assert!(session.page().is_none());
    assert_eq!(reading_state(), before_reading);
    load(&router, &mut session, fixture.base);
    assert!(session
        .page()
        .unwrap()
        .tokens
        .iter()
        .any(|token| token.surface == "𠮷か\u{3099}\0"));
    assert!(fixture
        .storage()
        .vocabulary_entry(&Surface::new("𠮷").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn effective_worker_override_rejects_stale_render_and_invalid_span_without_dictionary_or_writes() {
    use yomoku_core::{
        effective_overrides::EffectiveOverrideFailure as Failure,
        effective_session::EffectiveOverrideProgress as Progress,
    };
    let fixture = Fixture::new();
    let router = fixture.override_router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let identity = session.page().unwrap().identity;
    // A punctuation/Latin span is visible but not a valid boundary-edit target.
    let text = &fixture.imported.document.chapter.text;
    let start = text.find("English").unwrap();
    let anchor = TextAnchor {
        range: TextRange {
            start: TextOffset::at(text, start).unwrap(),
            end: TextOffset::at(text, start + "English".len()).unwrap(),
        },
        ..fixture.base.page
    };
    let request = session.begin_override_load(identity, anchor).unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let StorageResult::Effective(output) = run(&router, request.operation()) else {
        panic!("invalid")
    };
    assert_eq!(
        session.finish_override_request(&request, Ok(output)),
        Some(Progress::Failed(Failure::InvalidSelection))
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    load(&router, &mut session, fixture.base);
    let identity = session.page().unwrap().identity;
    let anchor = TextAnchor {
        range: TextRange {
            start: 0.try_into().unwrap(),
            end: 9.try_into().unwrap(),
        },
        ..fixture.base.page
    };
    let request = session.begin_override_load(identity, anchor).unwrap();
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let StorageResult::Effective(output) = run(&router, request.operation()) else {
        panic!("stale")
    };
    assert_eq!(
        session.finish_override_request(&request, Ok(output)),
        Some(Progress::Failed(Failure::StaleContext))
    );
    assert!(session.page().is_none());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
}

#[test]
fn effective_worker_retained_status_rolls_back_retries_and_never_loads_dictionaries() {
    let fixture = Fixture::new();
    let mut storage = fixture.storage();
    storage
        .register_vocabulary(&[Surface::new("食べ").unwrap()], 110)
        .unwrap();
    let id = storage
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .unwrap()
        .id;
    let original = storage
        .chapter_analysis(fixture.imported.analysis_id)
        .unwrap();
    let reader = storage
        .load_document_reader_snapshot(fixture.base.document_id)
        .unwrap();
    drop(storage);
    let router = || {
        let (effects, _) = StorageEffects::with_dictionaries(
            fixture.path(),
            Box::new(|| panic!("manual statuses must not load Sudachi")),
            Box::new(|| panic!("manual statuses must not open the word dictionary")),
        )
        .unwrap();
        Core::<Probe>::new().handle_effects_using(effects)
    };
    let worker = router();
    let mut session = EffectiveReaderSession::default();
    load(&worker, &mut session, fixture.base);
    let selection = select(&session, "食べ").selection();
    let before = session.page().unwrap().clone();
    let events = || {
        fixture
            .sql()
            .query_row(
                "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
    };
    fixture.sql().execute_batch("CREATE TRIGGER fail_status AFTER INSERT ON vocabulary_status_events WHEN NEW.cause = 'manual' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let request = session
        .begin_status_change(selection, id, VocabularyStatus::Known, 500)
        .unwrap();
    let StorageResult::Failed(error) = run(&worker, request.operation()) else {
        panic!("expected injected failure")
    };
    assert!(session.finish_status_request(&request, Err(error)));
    assert!(session.status_state().pending);
    assert_eq!(session.page(), Some(&before));
    assert_eq!(events(), 0);
    assert_eq!(
        fixture
            .storage()
            .vocabulary_entry(&Surface::new("食べ").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    drop(worker);
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_status")
        .unwrap();
    let worker = router();
    let retry = session.retry_status_change().unwrap();
    assert_ne!(request, retry);
    assert_eq!(request.operation(), retry.operation());
    let StorageResult::Effective(result) = run(&worker, retry.operation()) else {
        panic!("expected success")
    };
    // Lose the successful response and require an exact explicit retry.
    assert!(matches!(result, Output::StatusChanged(..)));
    assert!(session.finish_status_request(&retry, Err(DocumentError::Unavailable)));
    let replay = session.retry_status_change().unwrap();
    assert_eq!(replay.operation(), retry.operation());
    assert!(!session.finish_status_request(&retry, Ok(result)));
    let StorageResult::Effective(result) = run(&worker, replay.operation()) else {
        panic!("expected no-op retry")
    };
    assert!(session.finish_status_request(&replay, Ok(result)));
    assert_eq!(events(), 1);
    assert!(!session.status_state().pending);
    let repeats: Vec<_> = session
        .page()
        .unwrap()
        .tokens
        .iter()
        .filter(|t| t.surface == "食べ")
        .collect();
    assert_eq!(repeats.len(), 2);
    assert!(repeats
        .iter()
        .all(|t| t.vocabulary.unwrap().status == VocabularyStatus::Known
            && t.reading.is_none()
            && t.dictionary_form.is_none()));

    let stale = session
        .begin_status_change(selection, id, VocabularyStatus::Learning, 501)
        .unwrap();
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    session.invalidate();
    assert_eq!(
        run(&worker, stale.operation()),
        StorageResult::Failed(DocumentError::InvalidDocument)
    );
    assert!(session.finish_status_request(&stale, Err(DocumentError::InvalidDocument)));
    assert!(session.page().is_none());
    assert!(session.cancel_status_retry());
    assert_eq!(events(), 1);
    let mut storage = fixture.storage();
    assert_eq!(
        storage
            .chapter_analysis(fixture.imported.analysis_id)
            .unwrap(),
        original
    );
    assert_eq!(
        storage
            .load_document_reader_snapshot(fixture.base.document_id)
            .unwrap(),
        reader
    );
    for table in [
        "reading_completions",
        "reading_occurrences",
        "pending_reading_time",
        "completion_commands",
        "effective_completion_commands",
    ] {
        assert_eq!(
            fixture
                .sql()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn effective_worker_retained_word_lifecycle_compacts_results_and_retries_frozen_custom_commands() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let cat = select(&session, "猫").selection();
    let entry = session
        .page()
        .unwrap()
        .tokens
        .iter()
        .find(|token| token.surface == "猫")
        .unwrap()
        .vocabulary
        .unwrap();
    fixture
        .storage()
        .set_vocabulary_status(entry.id, VocabularyStatus::Ignored, 110)
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let request = session.begin_word_load(cat).unwrap();
    finish_word(&router, &mut session, &request);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert_eq!(
        session
            .word_state()
            .details
            .as_ref()
            .unwrap()
            .candidates
            .words
            .len(),
        1
    );
    assert_eq!(
        session
            .word_state()
            .details
            .as_ref()
            .unwrap()
            .token
            .vocabulary
            .unwrap()
            .status,
        VocabularyStatus::Ignored
    );
    assert!(session
        .begin_custom_entry_save(cat, "replace built-in".into(), "cat".into())
        .is_none());
    assert!(session.begin_custom_entry_delete(cat).is_none());
    fixture
        .sql()
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(entry.id)],
        )
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    let request = session.begin_word_load(cat).unwrap();
    finish_word(&router, &mut session, &request);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert!(session
        .word_state()
        .details
        .as_ref()
        .unwrap()
        .token
        .vocabulary
        .is_none());

    let manual = select(&session, "食べ").selection();
    let request = session.begin_word_load(manual).unwrap();
    finish_word(&router, &mut session, &request);
    let details = session.word_state().details.unwrap();
    assert!(details.token.reading.is_none() && details.token.dictionary_form.is_none());
    assert!(details.candidates.dictionary_form.is_none());
    fixture.sql().execute_batch("CREATE TRIGGER fail_custom BEFORE INSERT ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let request = session
        .begin_custom_entry_save(
            manual,
            " literal\0meaning 📖 ".into(),
            " よみか\u{3099}\0 ".into(),
        )
        .unwrap();
    finish_word(&router, &mut session, &request);
    assert!(session.word_state().custom_entry_error.is_some());
    assert!(session.word_state().details.unwrap().custom_entry.is_none());
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_custom")
        .unwrap();
    let retry = session.retry_custom_entry().unwrap();
    assert_ne!(retry, request);
    assert_eq!(retry.operation(), request.operation());
    finish_word(&router, &mut session, &retry);
    let saved = session.word_state().details.unwrap().custom_entry.unwrap();
    assert_eq!(saved.surface, "食べ");
    assert_eq!(saved.english_meaning, " literal\0meaning 📖 ");
    assert_eq!(saved.reading, " よみか\u{3099}\0 ");
    assert!(session
        .page()
        .unwrap()
        .tokens
        .iter()
        .filter(|token| token.surface == "食べ")
        .all(|token| token.vocabulary.is_none()));
    let delete = session.begin_custom_entry_delete(manual).unwrap();
    session.invalidate();
    finish_word(&router, &mut session, &delete);
    assert!(!session.word_state().custom_entry_pending);
    assert!(session.word_state().details.is_none());
    assert!(word(
        &router,
        match delete.operation() {
            Op::DeleteCustomEntry(command) => command,
            _ => unreachable!(),
        }
    )
    .custom_entry
    .is_none());
    let sql = fixture.sql();
    for table in [
        "reading_completions",
        "reading_occurrences",
        "pending_reading_time",
        "completion_commands",
    ] {
        let count: i64 = sql
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
}

#[test]
fn effective_worker_reads_exact_manual_metadata_live_status_and_literal_custom_entries() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    let before = std::fs::read(fixture.path()).unwrap();
    load(&router, &mut session, fixture.base);
    let manual = select(&session, "食べ");
    let details = word(&router, manual.clone());
    assert!(details.token.metadata.is_none());
    assert!(details.token.vocabulary.is_none()); // reads never register override parts
    assert!(details.candidates.dictionary_form.is_none());
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
    let save = Op::SaveCustomEntry {
        command: command.clone(),
        english_meaning: " literal\0meaning 📖 ".into(),
        reading: " よみ\u{3099}\0 ".into(),
    };
    let StorageResult::Effective(Output::CustomEntryChanged(selection, Some(entry))) =
        run(&router, save.clone())
    else {
        panic!("expected custom entry")
    };
    assert_eq!(selection, command.selection());
    assert_eq!(entry.english_meaning, " literal\0meaning 📖 ");
    assert_eq!(entry.reading, " よみ\u{3099}\0 ");
    assert_eq!(
        word(&router, command.clone()).custom_entry,
        Some(entry.clone())
    );
    let before = std::fs::read(fixture.path()).unwrap();
    assert!(matches!(
        run(&router, save),
        StorageResult::Effective(Output::CustomEntryChanged(_, Some(_)))
    ));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert_eq!(
        run(
            &router,
            Op::SaveCustomEntry {
                command: cat.clone(),
                english_meaning: "replace".into(),
                reading: "replace".into()
            }
        ),
        StorageResult::Failed(DocumentError::InvalidDocument)
    );
    assert_eq!(
        run(&router, Op::DeleteCustomEntry(cat)),
        StorageResult::Failed(DocumentError::InvalidDocument)
    );
    assert!(matches!(
        run(&router, Op::DeleteCustomEntry(command.clone())),
        StorageResult::Effective(Output::CustomEntryChanged(_, None))
    ));
    assert!(word(&router, command).custom_entry.is_none());
    // Session invalidation retires callbacks while accepted commands keep their key.
    session.invalidate();
    assert_eq!(word(&router, manual).token.surface, "食べ");
}

#[test]
fn effective_worker_revalidates_stale_keys_before_word_custom_and_completion_commands() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let command = select(&session, "食べ");
    let page = session
        .freeze_page(session.page().unwrap().identity)
        .unwrap();
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    for operation in [
        Op::LoadWord(command.clone()),
        Op::SaveCustomEntry {
            command: command.clone(),
            english_meaning: "stale".into(),
            reading: "stale".into(),
        },
        Op::DeleteCustomEntry(command),
        Op::PrepareCompletion {
            intent: fixture.intent,
            command: page,
        },
    ] {
        assert_eq!(
            run(&router, operation),
            StorageResult::Failed(DocumentError::InvalidDocument)
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    }
    load(&router, &mut session, fixture.base);
    let details = word(&router, select(&session, "食べた"));
    assert_eq!(
        details.token.metadata.unwrap().reading.as_deref(),
        Some("読み食べた")
    );
}

#[test]
fn effective_worker_dictionary_failures_require_explicit_retry_and_reopen_the_index() {
    let fixture = Fixture::new();
    let attempts = Arc::new(AtomicUsize::new(0));
    let trace = Arc::clone(&attempts);
    let index = fixture.index();
    let calling_thread = thread::current().id();
    let (effects, _) = StorageEffects::with_dictionaries(
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
    let before = std::fs::read(fixture.path()).unwrap();
    for operation in [
        Op::SaveCustomEntry {
            command: command.clone(),
            english_meaning: "must not overwrite a corrupt built-in".into(),
            reading: "読み".into(),
        },
        Op::DeleteCustomEntry(command.clone()),
    ] {
        assert_eq!(
            run(&router, operation),
            StorageResult::Failed(DocumentError::Unavailable)
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    }
    index
        .execute("UPDATE entries SET payload = ?1", [payload])
        .unwrap();
    assert_eq!(word(&router, command), expected);
    assert_eq!(attempts.load(Ordering::Relaxed), 5);
}

#[test]
fn effective_worker_missing_foreign_and_deleted_pages_fail_without_analysis_or_registration() {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    let legacy = fixture
        .storage()
        .create_paste(&PasteText::new("猫".into()).unwrap())
        .unwrap();
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
fn effective_worker_fifo_freezes_drained_time_then_retries_exact_completion_after_reversal_and_reopen(
) {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let page = session
        .freeze_page(session.page().unwrap().identity)
        .unwrap();
    let saved = fixture
        .storage()
        .load_pending_reading_time(fixture.base.document_id, fixture.intent.departing_position)
        .unwrap();
    // Keep the first operation blocked while both requests are submitted. The
    // same worker must save the final checkpoint before reserving its total.
    let lock = fixture.sql();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let checkpoint = enqueue(
        &router,
        StorageOperation::SaveReadingTime {
            previous: saved,
            checkpoint: ReadingTimeUpdate {
                pending: PendingReadingTime::from_millis(1234).unwrap(),
                ..saved.value
            },
        },
    );
    let preparation = enqueue(
        &router,
        StorageOperation::Effective(Op::PrepareCompletion {
            intent: fixture.intent,
            command: page,
        }),
    );
    assert!(checkpoint.try_recv().is_err());
    assert!(preparation.try_recv().is_err());
    session.invalidate();
    lock.execute_batch("COMMIT").unwrap();
    wait(checkpoint);
    wait(preparation);
    let StorageResult::Effective(Output::CompletionPrepared(prepared)) =
        router.view().pop().unwrap()
    else {
        panic!("expected reservation")
    };
    assert_eq!(prepared.completion().pending_time.milliseconds(), 1234);
    assert_eq!(
        prepared
            .occurrences()
            .iter()
            .map(|token| token.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た", "食べ", "た", "猫", "𠮷か\u{3099}\0"]
    );
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    fixture.sql().execute_batch("CREATE TRIGGER fail_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    assert_eq!(
        run(&router, Op::CompletePage(Arc::clone(&prepared))),
        StorageResult::Failed(DocumentError::Unavailable)
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_completion")
        .unwrap();
    // Replacing the worker does not auto-replay its durable reservation.
    drop(router);
    let router = fixture.router();
    wait(enqueue(&router, StorageOperation::List));
    assert!(matches!(
        router.view().last(),
        Some(StorageResult::Listed(..))
    ));
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let StorageResult::Effective(Output::PageCompleted(document, outcome)) =
        run(&router, Op::CompletePage(Arc::clone(&prepared)))
    else {
        panic!("expected completion")
    };
    assert_eq!(
        outcome,
        AnalyzedCompletionOutcome::Recorded { newly_known: 4 }
    );
    assert!(document.finished_at_least_once);
    assert_eq!(
        document.current_position().unwrap().offset,
        fixture.base.page.range.end
    );
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        1234
    );
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        6
    );
    let before = std::fs::read(fixture.path()).unwrap();
    assert!(matches!(
        run(&router, Op::CompletePage(prepared)),
        StorageResult::Effective(Output::PageCompleted(
            _,
            AnalyzedCompletionOutcome::AlreadyRecorded
        ))
    ));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    fixture
        .storage()
        .save_document_reading_position(fixture.base.document_id, fixture.intent.departing_position)
        .unwrap();
    load(&router, &mut session, fixture.base);
    let command = session
        .freeze_page(session.page().unwrap().identity)
        .unwrap();
    let StorageResult::Effective(Output::CompletionPrepared(reread)) = run(
        &router,
        Op::PrepareCompletion {
            intent: fixture.intent,
            command,
        },
    ) else {
        panic!("expected reread reservation")
    };
    assert_eq!(reread.occurrences().len(), 4); // reversed rule uses original B now
    assert!(matches!(
        run(&router, Op::CompletePage(reread)),
        StorageResult::Effective(Output::PageCompleted(
            _,
            AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
        ))
    ));
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        10
    );
    // The immutable import snapshot still has Mode B and its exact original words.
    assert_eq!(
        fixture
            .storage()
            .chapter_analysis(fixture.imported.analysis_id)
            .unwrap()
            .unwrap()
            .mode,
        SplitMode::B
    );
}

fn finish_completion(
    router: &Router,
    session: &mut EffectiveReaderSession,
    request: &EffectiveCompletionRequest,
) -> CompletionProgress {
    let result = match run(router, request.operation()) {
        StorageResult::Effective(result) => Ok(result),
        StorageResult::Failed(error) => Err(error),
        result => panic!("unexpected completion result: {result:?}"),
    };
    session.finish_completion_request(request, result).unwrap()
}

#[test]
fn effective_worker_retained_completion_drains_time_and_retries_without_reinterpreting_or_double_counting(
) {
    let fixture = Fixture::new();
    let router = fixture.router();
    let mut session = EffectiveReaderSession::default();
    load(&router, &mut session, fixture.base);
    let identity = session.page().unwrap().identity;
    assert!(session.begin_completion(identity, fixture.intent));
    assert!(session.completion_state().pending && !session.completion_state().busy);
    assert!(!session.begin_completion(identity, fixture.intent));

    let saved = fixture
        .storage()
        .load_pending_reading_time(fixture.base.document_id, fixture.intent.departing_position)
        .unwrap();
    let lock = fixture.sql();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    let checkpoint = enqueue(
        &router,
        StorageOperation::SaveReadingTime {
            previous: saved,
            checkpoint: ReadingTimeUpdate {
                pending: PendingReadingTime::from_millis(1234).unwrap(),
                ..saved.value
            },
        },
    );
    assert!(checkpoint.try_recv().is_err());
    // Accepted completion owns its old page while final clock work is blocked.
    session.invalidate();
    assert!(!session.completion_state().busy);
    lock.execute_batch("COMMIT").unwrap();
    wait(checkpoint);
    let StorageResult::ReadingTimeSaved(saved) = router.view().pop().unwrap() else {
        panic!("saved checkpoint")
    };
    // The app drains its coalesced maximum before dispatching preparation.
    wait(enqueue(
        &router,
        StorageOperation::SaveReadingTime {
            previous: saved,
            checkpoint: ReadingTimeUpdate {
                pending: PendingReadingTime::from_millis(2345).unwrap(),
                ..saved.value
            },
        },
    ));
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM completion_commands", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    fixture.sql().execute_batch("CREATE TRIGGER fail_preparation BEFORE INSERT ON completion_commands BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let prepare = session.dispatch_completion().unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    assert_eq!(
        finish_completion(&router, &mut session, &prepare),
        CompletionProgress::Failed(DocumentError::Unavailable)
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_preparation")
        .unwrap();
    let retry = session.dispatch_completion().unwrap();
    assert_ne!(retry, prepare);
    assert_eq!(retry.operation(), prepare.operation());
    assert!(session
        .finish_completion_request(&prepare, Err(DocumentError::Unavailable))
        .is_none());
    assert_eq!(
        finish_completion(&router, &mut session, &retry),
        CompletionProgress::Prepared
    );
    assert!(session
        .finish_completion_request(&retry, Err(DocumentError::Unavailable))
        .is_none());
    let commit = session.dispatch_completion().unwrap();
    let Op::CompletePage(prepared) = commit.operation() else {
        panic!("commit reservation")
    };
    assert_eq!(prepared.completion().pending_time.milliseconds(), 2345);
    assert_eq!(prepared.occurrences().len(), 6);
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    fixture.sql().execute_batch("CREATE TRIGGER fail_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let before = std::fs::read(fixture.path()).unwrap();
    assert_eq!(
        finish_completion(&router, &mut session, &commit),
        CompletionProgress::Failed(DocumentError::Unavailable)
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_completion")
        .unwrap();
    drop(router);
    let router = fixture.router();
    wait(enqueue(&router, StorageOperation::List));
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let retry = session.dispatch_completion().unwrap();
    let Op::CompletePage(retried) = retry.operation() else {
        panic!("exact retry")
    };
    assert!(Arc::ptr_eq(&prepared, &retried));
    assert!(session
        .finish_completion_request(&commit, Err(DocumentError::Unavailable))
        .is_none());
    // Simulate a committed transaction whose follow-up read/response was lost.
    assert!(matches!(
        run(&router, retry.operation()),
        StorageResult::Effective(Output::PageCompleted(
            _,
            AnalyzedCompletionOutcome::Recorded { newly_known: 4 }
        ))
    ));
    assert_eq!(
        session.finish_completion_request(&retry, Err(DocumentError::Unavailable)),
        Some(CompletionProgress::Failed(DocumentError::Unavailable))
    );
    let final_retry = session.dispatch_completion().unwrap();
    assert_eq!(retry.operation(), final_retry.operation());
    let CompletionProgress::Completed {
        completion,
        document,
        outcome,
    } = finish_completion(&router, &mut session, &final_retry)
    else {
        panic!("completed")
    };
    assert_eq!(completion, prepared.completion());
    assert_eq!(outcome, AnalyzedCompletionOutcome::AlreadyRecorded);
    assert!(document.finished_at_least_once);
    assert_eq!(
        document.current_position().unwrap().offset,
        fixture.base.page.range.end
    );
    assert!(!session.completion_state().pending);
    assert!(session.dispatch_completion().is_none());
    assert!(session
        .finish_completion_request(&final_retry, Err(DocumentError::Unavailable))
        .is_none());
    let sql = fixture.sql();
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        sql.query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        2345
    );
    assert_eq!(
        sql.query_row(
            "SELECT SUM(milliseconds) FROM pending_reading_time",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(EffectiveReaderSession::default()
        .dispatch_completion()
        .is_none());
}
