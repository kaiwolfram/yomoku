use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use crux_core::{App, Command, Core};
use rusqlite::Connection;
use yomoku_core::{
    anki::{
        AnkiModelCreation, AnkiModelId, AnkiModelLayout, AnkiNote, AnkiNoteId, AnkiRecoveryState,
        AnkiSubmission,
    },
    app::{Effect, Model, ViewModel, Yomoku},
    document::{TextOffset, TextRange},
    storage::DocumentError,
    vocabulary::Surface,
};
use yomoku_storage::{anki::AnkiSubmissionOutcome, anki_model::AnkiModelCreationOutcome, Storage};

use super::*;
use crate::storage::StorageEffects;

#[derive(Default)]
struct Probe {
    lose_commit_response: bool,
}
impl App for Probe {
    type Event = Event;
    type Model = Model;
    type ViewModel = (ViewModel, AnkiRecoveryState, bool, Option<DocumentError>);
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        let event = match event {
            Event::AnkiRecoveryCommitFinished(
                id,
                yomoku_core::storage::StorageResult::AnkiRecoveryCommitted(_),
            ) if self.lose_commit_response => Event::AnkiRecoveryCommitFinished(
                id,
                yomoku_core::storage::StorageResult::Failed(DocumentError::Unavailable),
            ),
            event => event,
        };
        Yomoku.update(event, model)
    }

    fn view(&self, model: &Model) -> Self::ViewModel {
        (
            Yomoku.view(model),
            model.anki_recovery().clone(),
            model.anki_recovery_saving(),
            model.anki_recovery_commit_error(),
        )
    }
}

type Router =
    AnkiStartupLayer<crux_core::middleware::HandleEffectLayer<Core<Probe>, StorageEffects>>;
struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "yomoku-anki-startup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("user.sqlite3")
    }
    fn router(&self, effective: bool) -> Router {
        self.router_with_loss(effective, false)
    }
    fn router_with_loss(&self, effective: bool, lose_commit_response: bool) -> Router {
        let (worker, _) = StorageEffects::with_dictionaries(
            self.path(),
            Box::new(|| panic!("startup must never load analyzer")),
            Box::new(|| panic!("startup must never load dictionaries")),
        )
        .unwrap();
        AnkiStartupLayer::new(
            Core::new_with(
                Probe {
                    lose_commit_response,
                },
                if effective {
                    Model::with_effective_reader()
                } else {
                    Model::default()
                },
            )
            .handle_effects_using(worker),
        )
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn refresh(router: &Router) -> (ViewModel, AnkiRecoveryState) {
    let (sender, receiver) = mpsc::channel();
    router.update(Event::RefreshLibrary, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, Effect::Render(_))));
        sender.send(std::thread::current().id()).unwrap();
    });
    assert_ne!(
        receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
        std::thread::current().id()
    );
    let view = router.view();
    assert!(!view.0.storage_busy);
    (view.0, view.1)
}

mod reconciliation;

fn submission() -> AnkiSubmission {
    let surface = Surface::new("𠮷猫").unwrap();
    let text = "𠮷猫。";
    let note = AnkiNote::new(
        &surface,
        "ねこ",
        &["cat & feline"],
        text,
        TextRange {
            start: TextOffset::START,
            end: TextOffset::at(text, surface.as_str().len()).unwrap(),
        },
    )
    .unwrap();
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* old frozen layout */");
    AnkiSubmission::new(
        note,
        9007199254740993_i64.try_into().unwrap(),
        i64::MAX.try_into().unwrap(),
        layout,
    )
    .unwrap()
}

#[test]
fn anki_startup_is_lazy_worker_only_and_retained_across_library_loads() {
    for effective in [false, true] {
        let database = Database::new();
        let router = database.router(effective);
        assert_eq!(router.view().1, AnkiRecoveryState::Unrestored);
        assert!(!database.path().exists());
        let (sender, receiver) = mpsc::channel();
        router.update(Event::CreatePaste(" \r\n".into()), move |_| {
            sender.send(()).unwrap();
        });
        receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(!database.path().exists());
        let (view, state) = refresh(&router);
        assert_eq!(view.document_error, None);
        assert!(state.allows_preparation());
        assert_eq!(refresh(&router).1, state);
        // Fresh core sessions must restore again, never inherit an empty gate.
        let restarted = database.router(effective);
        assert!(!restarted.view().1.allows_preparation());
        assert!(refresh(&restarted).1.allows_preparation());
    }
}

#[test]
fn anki_startup_restores_note_after_lost_callback_and_preserves_frozen_payload() {
    for uncertain in [false, true] {
        let database = Database::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let payload = submission();
        let live = storage.begin_anki_submission(&payload).unwrap();
        let known = AnkiNoteId::try_from(i64::MAX).unwrap();
        if uncertain {
            storage
                .finish_anki_submission(
                    &live,
                    AnkiSubmissionOutcome::Uncertain {
                        known_note_id: Some(known),
                    },
                )
                .unwrap();
        }
        let expected = storage.anki_recovery_snapshot().unwrap();
        drop(storage);
        let before = std::fs::read(database.path()).unwrap();
        for effective in [false, true] {
            let router = database.router(effective);
            let (view, state) = refresh(&router);
            assert_eq!(
                state,
                AnkiRecoveryState::Restored(Box::new(expected.clone()))
            );
            assert!(!state.allows_preparation());
            assert!(view.documents.is_empty());
            assert_eq!(refresh(&router).1, state);
        }
        assert_eq!(std::fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn anki_startup_restores_partial_model_and_never_replays_creation() {
    for uncertain in [false, true] {
        let database = Database::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let mut layout = AnkiModelLayout::yomoku();
        layout.name = "Frozen old 𠮷 model".into();
        let creation = AnkiModelCreation::new(layout).unwrap();
        let live = storage.begin_anki_model_creation(&creation).unwrap();
        if uncertain {
            storage
                .finish_anki_model_creation(
                    &live,
                    AnkiModelCreationOutcome::Uncertain {
                        known_model_id: Some(AnkiModelId::try_from(i64::MAX).unwrap()),
                    },
                )
                .unwrap();
        }
        let expected = storage.anki_recovery_snapshot().unwrap();
        drop(storage);
        let before = std::fs::read(database.path()).unwrap();
        let router = database.router(true);
        let (_, state) = refresh(&router);
        assert_eq!(state, AnkiRecoveryState::Restored(Box::new(expected)));
        assert!(!state.allows_preparation());
        assert_eq!(std::fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn anki_startup_restore_errors_do_not_break_library_and_only_explicit_retry_recovers() {
    for table in ["anki_submissions", "anki_model_creations"] {
        let database = Database::new();
        Storage::open(database.path()).unwrap();
        let sql = Connection::open(database.path()).unwrap();
        sql.execute(&format!("ALTER TABLE {table} RENAME TO saved_journal"), [])
            .unwrap();
        let router = database.router(true);
        let (view, state) = refresh(&router);
        assert_eq!(view.document_error, None);
        assert_eq!(state, AnkiRecoveryState::Failed(DocumentError::Unavailable));
        sql.execute(&format!("ALTER TABLE saved_journal RENAME TO {table}"), [])
            .unwrap();
        assert_eq!(refresh(&router).1, state); // no implicit retry
        router.update(Event::RetryAnkiRecovery, |_| {});
        assert!(refresh(&router).1.allows_preparation()); // FIFO barrier after Retry
    }
}
