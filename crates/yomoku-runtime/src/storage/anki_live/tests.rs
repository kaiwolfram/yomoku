use super::*;
use crate::storage::StorageEffects;
use crux_core::{
    middleware::{HandleEffectLayer, Layer},
    App, Command, Core,
};
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    anki::{
        AnkiExport, AnkiLiveInvocation, AnkiLiveOutcome, AnkiLiveProgress,
        AnkiModelCreationOutcome, AnkiPreflightDeck, AnkiPreflightModel, AnkiPreflightObservation,
        AnkiPreflightResult, AnkiRecoveryState, AnkiSelection, AnkiSubmissionOutcome,
    },
    app::{Effect, Event, Model, ViewModel, Yomoku},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    reader::ReaderTokenKey,
};

const TIMEOUT: Duration = Duration::from_secs(10);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "yomoku-anki-owner-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let fixture = Self(directory);
        let mut storage = Storage::open(fixture.path()).unwrap();
        storage
            .create_analyzed_paste(
                &PasteText::new("𠮷猫。".into()).unwrap(),
                100,
                |chapter: &ChapterText| {
                    let boundary = TextOffset::at(&chapter.text, "𠮷猫".len()).unwrap();
                    Ok::<_, std::convert::Infallible>(ChapterAnalysis {
                        profile: AnalysisProfile {
                            engine_revision: "test@1".into(),
                            dictionary_revision: "test@1".into(),
                        },
                        mode: SplitMode::B,
                        tokens: vec![
                            AnalyzedToken {
                                anchor: TextAnchor {
                                    source_id: chapter.source_id,
                                    chapter_id: chapter.chapter_id,
                                    range: TextRange {
                                        start: TextOffset::START,
                                        end: boundary,
                                    },
                                },
                                surface: "𠮷猫".into(),
                                is_japanese: true,
                                reading: Some("ねこ".into()),
                                dictionary_form: None,
                            },
                            AnalyzedToken {
                                anchor: TextAnchor {
                                    source_id: chapter.source_id,
                                    chapter_id: chapter.chapter_id,
                                    range: TextRange {
                                        start: boundary,
                                        end: TextOffset::at(&chapter.text, chapter.text.len())
                                            .unwrap(),
                                    },
                                },
                                surface: "。".into(),
                                is_japanese: false,
                                reading: None,
                                dictionary_form: None,
                            },
                        ],
                    })
                },
            )
            .unwrap();
        Connection::open(fixture.index()).unwrap().execute_batch("PRAGMA application_id = 0x594f4449; PRAGMA user_version = 1;
            CREATE TABLE entries (source TEXT, entry_id INTEGER, payload TEXT, PRIMARY KEY(source, entry_id)) WITHOUT ROWID;
            CREATE TABLE forms (surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER, PRIMARY KEY(surface, source, entry_id)) WITHOUT ROWID;").unwrap();
        fixture
    }
    fn path(&self) -> PathBuf {
        self.0.join("user.sqlite3")
    }
    fn index(&self) -> PathBuf {
        self.0.join("words.sqlite3")
    }
    fn storage(&self) -> Storage {
        Storage::open(self.path()).unwrap()
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
    fn router(&self, effective: bool, lose_begin: bool, lose_outcome: bool) -> Harness {
        let index = self.index();
        let (worker, _) = StorageEffects::with_dictionaries(
            self.path(),
            Box::new(|| panic!("no analyzer")),
            Box::new(move || Ok(index.clone())),
        )
        .unwrap();
        let invocation = Arc::new(Mutex::new(None));
        let router = Core::new_with(
            Probe {
                invocation: invocation.clone(),
                lose_begin,
                lose_outcome,
            },
            if effective {
                Model::with_effective_reader()
            } else {
                Model::default()
            },
        )
        .handle_effects_using(worker);
        Harness { router, invocation }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Probe {
    invocation: Arc<Mutex<Option<AnkiLiveInvocation>>>,
    lose_begin: bool,
    lose_outcome: bool,
}
impl App for Probe {
    type Event = Event;
    type Model = Model;
    type ViewModel = (
        ViewModel,
        AnkiRecoveryState,
        Option<AnkiLiveProgress>,
        Option<AnkiExport>,
    );
    type Effect = Effect;
    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        let event = match event {
            Event::AnkiLiveBeginFinished(request, StorageResult::AnkiLiveBegun(_))
                if self.lose_begin =>
            {
                Event::AnkiLiveBeginFinished(
                    request,
                    StorageResult::Failed(DocumentError::Unavailable),
                )
            }
            Event::AnkiLiveOutcomeFinished(request, StorageResult::AnkiLiveFinished(_))
                if self.lose_outcome =>
            {
                Event::AnkiLiveOutcomeFinished(
                    request,
                    StorageResult::Failed(DocumentError::Unavailable),
                )
            }
            event => event,
        };
        let command = Yomoku.update(event, model);
        if let Some(invocation) = model.take_anki_live_invocation() {
            assert!(self
                .invocation
                .lock()
                .unwrap()
                .replace(invocation)
                .is_none());
        }
        command
    }
    fn view(&self, model: &Model) -> Self::ViewModel {
        (
            Yomoku.view(model),
            model.anki_recovery().clone(),
            model.anki_live_progress(),
            model.prepared_anki_export().cloned(),
        )
    }
}

type Router = HandleEffectLayer<Core<Probe>, StorageEffects>;
struct Harness {
    router: Router,
    invocation: Arc<Mutex<Option<AnkiLiveInvocation>>>,
}
impl Harness {
    fn dispatch(
        &self,
        event: Event,
        done: impl Fn(&<Probe as App>::ViewModel) -> bool,
    ) -> Vec<Effect> {
        let (sender, receiver) = mpsc::channel();
        let effects = self.router.update(event, move |_| {
            let _ = sender.send(std::thread::current().id());
        });
        let deadline = Instant::now() + TIMEOUT;
        while !done(&self.router.view()) {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(thread) => assert_ne!(thread, std::thread::current().id()),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    assert!(done(&self.router.view()), "native completion condition")
                }
                Err(error) => panic!("native completion timeout: {error}"),
            }
        }
        effects
    }
    fn local(&self, event: Event) -> Vec<Effect> {
        self.dispatch(event, |v| !v.0.storage_busy)
    }
    fn prime(&self, fixture: &Fixture, create: bool) {
        self.dispatch(Event::RestoreAnkiRecovery, |v| v.1.allows_preparation());
        self.local(Event::RefreshLibrary);
        let id = fixture.storage().list_documents().unwrap()[0].document_id;
        self.local(Event::OpenReader(id));
        let view = self.router.view().0;
        let page = view.reader.unwrap();
        self.local(Event::LoadReaderTokens(ReaderTokenKey {
            document_id: id,
            page: page.anchor,
            analysis: view.reader_analysis.unwrap(),
        }));
        let view = self.router.view().0;
        let selection = if let Some(page) = view.effective_reader_tokens {
            let selection = yomoku_core::effective_session::EffectiveReaderSelection {
                page: page.identity,
                segment_index: page.tokens[0].segment_index,
            };
            self.local(Event::LoadEffectiveReaderWord(selection));
            AnkiSelection::Effective(selection)
        } else {
            let tokens = view.reader_tokens.unwrap();
            let selection = yomoku_core::reader::ReaderWordSelection {
                key: tokens.key(),
                token_index: 0,
            };
            self.local(Event::LoadReaderWord(selection));
            AnkiSelection::Original(selection)
        };
        self.local(Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        });
        assert!(self.router.view().3.is_some());
        let mut effects = self.local(Event::BeginAnkiExportPreflight);
        let Some(Effect::AnkiExportPreflight(request)) = effects.first_mut() else {
            panic!("preflight")
        };
        self.router
            .resolve(
                request,
                AnkiPreflightResult::Complete(AnkiPreflightObservation {
                    decks: vec![AnkiPreflightDeck {
                        id: i64::MAX,
                        filtered: false,
                    }],
                    duplicate_note_ids: vec![],
                    models: if create {
                        vec![]
                    } else {
                        vec![AnkiPreflightModel {
                            id: 9007199254740993,
                            name: "Yomoku v1".into(),
                            matches_layout: true,
                        }]
                    },
                }),
                |_| {},
            )
            .unwrap();
    }
    fn begin(&self) {
        self.dispatch(Event::BeginAnkiLive, |v| {
            v.2 != Some(AnkiLiveProgress::Beginning)
        });
    }
    fn take(&self) -> AnkiLiveInvocation {
        self.invocation.lock().unwrap().take().unwrap()
    }
    fn finish(&self, callback: yomoku_core::anki::AnkiLiveCallback) {
        self.dispatch(Event::FinishAnkiLive(Box::new(callback)), |v| {
            v.2 != Some(AnkiLiveProgress::Saving)
        });
    }
    fn restore(&self) {
        self.dispatch(Event::RetryAnkiRecovery, |v| {
            !matches!(v.1, AnkiRecoveryState::Loading)
        });
    }
}

fn terminal(create: bool) -> AnkiLiveOutcome {
    if create {
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Created(
            i64::MAX.try_into().unwrap(),
        ))
    } else {
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Added(i64::MAX.try_into().unwrap()))
    }
}

#[test]
fn anki_live_fifo_app_owns_frozen_begin_and_committed_outcomes_for_both_readers() {
    for effective in [false, true] {
        for create in [false, true] {
            let fixture = Fixture::new();
            let harness = fixture.router(effective, false, false);
            harness.prime(&fixture, create);
            let before = harness.router.view().0;
            harness.begin();
            assert_eq!(harness.router.view().2, Some(AnkiLiveProgress::Invoking));
            assert_eq!(harness.router.view().0, before);
            let invocation = harness.take();
            let callback = invocation.complete(terminal(create));
            let snapshot = fixture.storage().anki_recovery_snapshot().unwrap();
            assert!(snapshot.note.is_some() || snapshot.model.is_some());
            harness.local(Event::BeginAnkiLive);
            assert!(harness.invocation.lock().unwrap().is_none());
            harness.finish(callback.clone());
            assert_eq!(harness.router.view().2, Some(AnkiLiveProgress::Finished));
            assert!(harness.router.view().1.allows_preparation());
            assert_eq!(harness.router.view().0, before);
            assert_eq!(
                fixture.storage().anki_recovery_snapshot().unwrap(),
                Default::default()
            );
            let bytes = std::fs::read(fixture.path()).unwrap();
            harness.finish(callback);
            assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
            assert!(harness.invocation.lock().unwrap().is_none());
        }
    }
}

#[test]
fn anki_live_fifo_lost_begin_and_outcome_responses_restore_without_new_authority() {
    for create in [false, true] {
        for lose_begin in [false, true] {
            let fixture = Fixture::new();
            let harness = fixture.router(true, lose_begin, !lose_begin);
            harness.prime(&fixture, create);
            harness.begin();
            if lose_begin {
                assert!(harness.invocation.lock().unwrap().is_none());
                assert_ne!(
                    fixture.storage().anki_recovery_snapshot().unwrap(),
                    Default::default()
                );
            } else {
                let callback = harness.take().complete(terminal(create));
                harness.finish(callback);
                assert_eq!(
                    fixture.storage().anki_recovery_snapshot().unwrap(),
                    Default::default()
                );
            }
            assert_eq!(
                harness.router.view().2,
                Some(AnkiLiveProgress::Failed(DocumentError::Unavailable))
            );
            harness.restore();
            assert_eq!(harness.router.view().1.allows_preparation(), !lose_begin);
            assert!(harness.router.view().3.is_none());
            harness.local(Event::BeginAnkiLive);
            assert!(harness.invocation.lock().unwrap().is_none());
            let restarted = fixture.router(true, false, false);
            restarted.dispatch(Event::RestoreAnkiRecovery, |v| {
                !matches!(v.1, AnkiRecoveryState::Loading)
            });
            assert_eq!(restarted.router.view().1, harness.router.view().1);
            assert!(restarted.invocation.lock().unwrap().is_none());
        }
    }
}

#[test]
fn anki_live_fifo_statement_and_commit_failures_roll_back_and_never_replay() {
    for create in [false, true] {
        for finish in [false, true] {
            for deferred in [false, true] {
                let fixture = Fixture::new();
                let harness = fixture.router(false, false, false);
                harness.prime(&fixture, create);
                if finish {
                    harness.begin();
                }
                let before = fixture.storage().anki_recovery_snapshot().unwrap();
                let sql = fixture.sql();
                sql.execute_batch("CREATE TABLE deferred_live_owner (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
                let action = if deferred {
                    "INSERT INTO deferred_live_owner VALUES (-1);"
                } else {
                    "SELECT RAISE(ABORT, 'fixture');"
                };
                let table = if create {
                    "anki_model_creations"
                } else {
                    "anki_submissions"
                };
                let verb = if finish { "UPDATE" } else { "INSERT" };
                sql.execute_batch(&format!(
                    "CREATE TRIGGER fail_live_owner AFTER {verb} ON {table} BEGIN {action} END;"
                ))
                .unwrap();
                if finish {
                    harness.finish(harness.take().complete(terminal(create)));
                } else {
                    harness.begin();
                }
                assert_eq!(
                    harness.router.view().2,
                    Some(AnkiLiveProgress::Failed(DocumentError::Unavailable))
                );
                assert_eq!(fixture.storage().anki_recovery_snapshot().unwrap(), before);
                sql.execute_batch("DROP TRIGGER fail_live_owner").unwrap();
                harness.local(Event::BeginAnkiLive);
                assert!(harness.invocation.lock().unwrap().is_none());
                harness.restore();
                assert_eq!(harness.router.view().1.allows_preparation(), !finish);
                assert!(harness.router.view().3.is_none());
                assert_eq!(harness.router.view().0.document_error, None);
            }
        }
    }
}

#[test]
fn anki_live_fifo_uncertain_known_ids_survive_abandon_close_and_reopen() {
    for create in [false, true] {
        let fixture = Fixture::new();
        let harness = fixture.router(true, false, false);
        harness.prime(&fixture, create);
        harness.begin();
        let outcome = if create {
            AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Uncertain {
                known_model_id: Some(i64::MAX.try_into().unwrap()),
            })
        } else {
            AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Uncertain {
                known_note_id: Some(i64::MAX.try_into().unwrap()),
            })
        };
        harness.finish(harness.take().complete(outcome));
        let snapshot = fixture.storage().anki_recovery_snapshot().unwrap();
        if create {
            assert_eq!(
                i64::from(snapshot.model.as_ref().unwrap().known_model_id.unwrap()),
                i64::MAX
            );
        } else {
            assert_eq!(
                i64::from(snapshot.note.as_ref().unwrap().known_note_id.unwrap()),
                i64::MAX
            );
        }
        assert!(!harness.router.view().1.allows_preparation());
        drop(harness);
        let reopened = fixture.router(true, false, false);
        reopened.dispatch(Event::RestoreAnkiRecovery, |v| {
            !matches!(v.1, AnkiRecoveryState::Loading)
        });
        assert_eq!(
            reopened.router.view().1,
            AnkiRecoveryState::Restored(Box::new(snapshot))
        );
        assert!(reopened.invocation.lock().unwrap().is_none());
    }
}

#[test]
fn anki_live_worker_rejects_foreign_finish_duplicate_begin_and_consumed_outcomes() {
    let fixture = Fixture::new();
    let harness = fixture.router(false, false, false);
    harness.prime(&fixture, false);
    harness.begin();
    let callback = harness.take().complete(terminal(false));
    // A different worker has no retained attempt, even for the same database.
    let mut foreign = AnkiLiveWorker::default();
    let mut storage = None;
    assert_eq!(
        foreign.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::FinishAnkiLive(Box::new(callback.clone()))
        ),
        Err(DocumentError::Unavailable)
    );
    harness.finish(callback.clone());
    // A fresh native worker starts with no replay history. Use it to check the
    // worker boundary directly; production never moves begin requests this way.
    let mut worker = AnkiLiveWorker::default();
    let request = callback.request().clone();
    let begin = StorageOperation::BeginAnkiLive(Box::new(request));
    assert!(matches!(
        worker.execute(&fixture.path(), &mut storage, begin.clone()),
        Ok(StorageResult::AnkiLiveBegun(_))
    ));
    assert_eq!(
        worker.execute(&fixture.path(), &mut storage, begin.clone()),
        Err(DocumentError::Unavailable)
    );
    assert!(matches!(
        worker.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::FinishAnkiLive(Box::new(callback.clone()))
        ),
        Ok(StorageResult::AnkiLiveFinished(_))
    ));
    assert_eq!(
        worker.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::FinishAnkiLive(Box::new(callback))
        ),
        Err(DocumentError::Unavailable)
    );
    assert_eq!(
        worker.execute(&fixture.path(), &mut storage, begin.clone()),
        Err(DocumentError::Unavailable)
    );
    worker
        .execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::LoadAnkiRecovery,
        )
        .unwrap();
    assert_eq!(
        worker.execute(&fixture.path(), &mut storage, begin),
        Err(DocumentError::Unavailable)
    );
}

#[test]
fn anki_live_fifo_blocked_commit_keeps_navigation_responsive_and_abandon_revokes_late_begin() {
    let fixture = Fixture::new();
    let harness = fixture.router(true, false, false);
    harness.prime(&fixture, false);
    let sql = fixture.sql();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(harness
        .router
        .update(Event::BeginAnkiLive, |_| {})
        .is_empty());
    assert_eq!(harness.router.view().2, Some(AnkiLiveProgress::Beginning));
    harness.local(Event::OpenSettings);
    assert!(harness.router.view().0.settings_open);
    assert!(!harness.router.view().0.storage_busy);
    harness.local(Event::AbandonAnkiLive);
    sql.execute_batch("ROLLBACK").unwrap();
    harness.restore(); // queued after begin; the late success cannot grant a permit
    assert!(!harness.router.view().1.allows_preparation());
    assert!(harness.invocation.lock().unwrap().is_none());
    assert!(harness.router.view().3.is_none());
    let pair = fixture.storage().anki_recovery_snapshot().unwrap();
    assert!(pair.note.is_some());
}

#[test]
fn anki_live_worker_failed_restore_discards_token_and_foreign_outcome_keeps_current_owner() {
    let fixture = Fixture::new();
    let harness = fixture.router(false, false, false);
    harness.prime(&fixture, false);
    harness.begin();
    let callback = harness.take().complete(terminal(false));
    harness.finish(callback.clone());
    let other = fixture.router(false, false, false);
    other.prime(&fixture, false);
    other.begin();
    let foreign = other.take().complete(terminal(false));
    other.finish(foreign.clone());
    let mut worker = AnkiLiveWorker::default();
    let mut storage = None;
    worker
        .execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::BeginAnkiLive(Box::new(callback.request().clone())),
        )
        .unwrap();
    let before = fixture.storage().anki_recovery_snapshot().unwrap();
    assert_eq!(
        worker.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::FinishAnkiLive(Box::new(foreign))
        ),
        Err(DocumentError::Unavailable)
    );
    assert!(worker.retained.is_some());
    assert_eq!(fixture.storage().anki_recovery_snapshot().unwrap(), before);
    let sql = fixture.sql();
    sql.execute_batch("ALTER TABLE anki_model_creations RENAME TO saved_live_models")
        .unwrap();
    assert_eq!(
        worker.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::LoadAnkiRecovery
        ),
        Err(DocumentError::Unavailable)
    );
    assert!(worker.retained.is_none());
    sql.execute_batch("ALTER TABLE saved_live_models RENAME TO anki_model_creations")
        .unwrap();
    assert_eq!(
        worker.execute(
            &fixture.path(),
            &mut storage,
            StorageOperation::FinishAnkiLive(Box::new(callback))
        ),
        Err(DocumentError::Unavailable)
    );
    assert_eq!(fixture.storage().anki_recovery_snapshot().unwrap(), before);
}

#[test]
fn anki_live_worker_external_changed_pair_refuses_outcome_and_consumes_attempt() {
    let fixture = Fixture::new();
    let harness = fixture.router(false, false, false);
    harness.prime(&fixture, false);
    harness.begin();
    let callback = harness.take().complete(terminal(false));
    fixture
        .sql()
        .execute("UPDATE anki_submissions SET payload = '{}'", [])
        .unwrap();
    harness.finish(callback);
    assert_eq!(
        harness.router.view().2,
        Some(AnkiLiveProgress::Failed(DocumentError::Unavailable))
    );
    assert!(harness.invocation.lock().unwrap().is_none());
    assert!(!harness.router.view().1.allows_preparation());
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT payload FROM anki_submissions", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "{}"
    );
}
