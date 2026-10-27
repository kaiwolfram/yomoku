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
    },
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::PageLength,
    reader::{ReaderAnalysis, ReaderPage, ReaderTokenKey},
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_storage::{AnalyzedPaste, Storage};

use super::StorageEffects;

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
