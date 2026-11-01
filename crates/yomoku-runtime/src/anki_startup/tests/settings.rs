use super::*;
use yomoku_core::{anki::AnkiSettingsPhase, storage::StorageResult};

struct SettingsProbe(bool);
impl App for SettingsProbe {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        if self.0
            && matches!(
                &event,
                Event::AnkiSettingsFinished(_, StorageResult::AnkiDefaultDeckSaved(_))
            )
        {
            // Actual committed response loss: retain Saving until explicit readback.
            return crux_core::render::render();
        }
        Yomoku.update(event, model)
    }

    fn view(&self, model: &Model) -> ViewModel {
        Yomoku.view(model)
    }
}

type SettingsRouter = crux_core::middleware::HandleEffectLayer<Core<SettingsProbe>, StorageEffects>;
fn router(database: &Database, effective: bool, lose_save: bool) -> SettingsRouter {
    let (worker, _) = StorageEffects::with_dictionaries(
        database.path(),
        Box::new(|| panic!("settings must not load analyzer")),
        Box::new(|| panic!("settings must not load dictionaries")),
    )
    .unwrap();
    let router = Core::new_with(
        SettingsProbe(lose_save),
        if effective {
            Model::with_effective_reader()
        } else {
            Model::default()
        },
    )
    .handle_effects_using(worker);
    router.update(Event::OpenSettings, |_| {});
    router
}

fn dispatch(router: &SettingsRouter, event: Event) -> ViewModel {
    let (sender, receiver) = mpsc::channel();
    let effects = router.update(event, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, Effect::Render(_))));
        sender.send(std::thread::current().id()).unwrap();
    });
    assert!(effects
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
    assert_ne!(
        receiver.recv_timeout(Duration::from_secs(10)).unwrap(),
        std::thread::current().id()
    );
    router.view()
}

#[test]
fn anki_settings_fifo_commits_reopens_clears_and_preserves_both_pending_journals() {
    for (effective, model_pending) in [(false, false), (true, false), (false, true), (true, true)] {
        let database = Database::new();
        let mut storage = Storage::open(database.path()).unwrap();
        if model_pending {
            storage
                .begin_anki_model_creation(
                    &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                )
                .unwrap();
        } else {
            storage.begin_anki_submission(&submission()).unwrap();
        }
        let pair = storage.anki_recovery_snapshot().unwrap();
        drop(storage);
        let core = router(&database, effective, false);
        assert_eq!(core.view().anki_settings.phase, AnkiSettingsPhase::Unloaded);
        let before = dispatch(&core, Event::LoadAnkiSettings);
        assert_eq!(before.anki_settings.phase, AnkiSettingsPhase::Ready);
        assert_eq!(before.anki_settings.default_deck_id, None);
        let mut after = dispatch(&core, Event::SetAnkiDefaultDeck(Some(i64::MAX)));
        assert_eq!(after.anki_settings.default_deck_id, Some(i64::MAX));
        after.anki_settings = before.anki_settings.clone();
        assert_eq!(after, before);
        drop(core);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.anki_recovery_snapshot().unwrap(), pair);
        assert_eq!(
            storage.anki_default_deck().unwrap().map(i64::from),
            Some(i64::MAX)
        );
        drop(storage);
        let core = router(&database, effective, false);
        assert_eq!(core.view().anki_settings.default_deck_id, None);
        assert_eq!(
            dispatch(&core, Event::LoadAnkiSettings)
                .anki_settings
                .default_deck_id,
            Some(i64::MAX)
        );
        assert_eq!(
            dispatch(&core, Event::SetAnkiDefaultDeck(None))
                .anki_settings
                .default_deck_id,
            None
        );
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.anki_recovery_snapshot().unwrap(), pair);
    }
}

#[test]
fn anki_settings_fifo_statement_and_commit_failures_require_readback_and_preserve_preference() {
    for deferred in [false, true] {
        let database = Database::new();
        Storage::open(database.path())
            .unwrap()
            .save_anki_default_deck(Some(1.try_into().unwrap()))
            .unwrap();
        let sql = Connection::open(database.path()).unwrap();
        sql.execute_batch(if deferred {
            "CREATE TABLE settings_parent(id INTEGER PRIMARY KEY); CREATE TABLE settings_child(id INTEGER REFERENCES settings_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_settings AFTER UPDATE ON anki_settings BEGIN INSERT INTO settings_child VALUES(1); END;"
        } else {
            "CREATE TRIGGER fail_settings BEFORE UPDATE ON anki_settings BEGIN SELECT RAISE(ABORT, 'settings failure'); END;"
        }).unwrap();
        let core = router(&database, true, false);
        dispatch(&core, Event::LoadAnkiSettings);
        let failed = dispatch(&core, Event::SetAnkiDefaultDeck(Some(i64::MAX)));
        assert_eq!(failed.anki_settings.phase, AnkiSettingsPhase::Failed);
        assert_eq!(failed.anki_settings.default_deck_id, Some(1));
        assert!(!failed.storage_busy && failed.document_error.is_none());
        assert_eq!(
            Storage::open(database.path())
                .unwrap()
                .anki_default_deck()
                .unwrap()
                .map(i64::from),
            Some(1)
        );
        sql.execute_batch("DROP TRIGGER fail_settings").unwrap();
        let ready = dispatch(&core, Event::RetryAnkiSettings);
        assert_eq!(ready.anki_settings.default_deck_id, Some(1));
        assert_eq!(ready.anki_settings.phase, AnkiSettingsPhase::Ready);
        assert_eq!(
            dispatch(&core, Event::SetAnkiDefaultDeck(Some(i64::MAX)))
                .anki_settings
                .default_deck_id,
            Some(i64::MAX)
        );
    }
}

#[test]
fn anki_settings_fifo_lost_save_response_reads_committed_value_without_repeating_update() {
    let database = Database::new();
    Storage::open(database.path()).unwrap();
    let sql = Connection::open(database.path()).unwrap();
    sql.execute_batch("CREATE TABLE settings_writes(id INTEGER); CREATE TRIGGER count_settings AFTER UPDATE ON anki_settings BEGIN INSERT INTO settings_writes VALUES(1); END;").unwrap();
    let core = router(&database, true, true);
    dispatch(&core, Event::LoadAnkiSettings);
    let lost = dispatch(&core, Event::SetAnkiDefaultDeck(Some(9007199254740993)));
    assert_eq!(lost.anki_settings.phase, AnkiSettingsPhase::Saving);
    assert_eq!(lost.anki_settings.default_deck_id, None);
    let recovered = dispatch(&core, Event::RetryAnkiSettings);
    assert_eq!(recovered.anki_settings.phase, AnkiSettingsPhase::Ready);
    assert_eq!(
        recovered.anki_settings.default_deck_id,
        Some(9007199254740993)
    );
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM settings_writes", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn anki_settings_fifo_corrupt_or_missing_preferences_fail_without_repair_or_reader_error() {
    for corrupt in [false, true] {
        let database = Database::new();
        Storage::open(database.path()).unwrap();
        let sql = Connection::open(database.path()).unwrap();
        sql.execute_batch(if corrupt {
            "PRAGMA ignore_check_constraints=ON; UPDATE anki_settings SET default_deck_id=0;"
        } else {
            "DELETE FROM anki_settings;"
        })
        .unwrap();
        let core = router(&database, true, false);
        let failed = dispatch(&core, Event::LoadAnkiSettings);
        assert_eq!(failed.anki_settings.phase, AnkiSettingsPhase::Failed);
        assert_eq!(
            failed.anki_settings.storage_error,
            Some(DocumentError::InvalidSettings)
        );
        assert_eq!(failed.document_error, None);
        assert!(!failed.storage_busy);
        assert_eq!(
            dispatch(&core, Event::RetryAnkiSettings).anki_settings,
            failed.anki_settings
        );
        assert_eq!(sql.query_row("SELECT COUNT(*) FROM anki_settings WHERE default_deck_id IS NULL OR default_deck_id > 0", [], |row| row.get::<_, i64>(0)).unwrap(), 0);
    }
}
