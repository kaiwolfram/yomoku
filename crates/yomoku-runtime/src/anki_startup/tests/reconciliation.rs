use super::*;
use yomoku_core::anki::{
    AnkiExistingNotes, AnkiModelReconciliation, AnkiReconciliation, AnkiRecoveryCommit,
    AnkiRecoveryObservation,
};

fn captured(router: &Router, observation: AnkiRecoveryObservation) -> Box<AnkiRecoveryCommit> {
    let AnkiRecoveryState::Restored(expected) = router.view().1 else {
        panic!("restored")
    };
    Box::new(AnkiRecoveryCommit {
        expected,
        observation,
    })
}
fn existing() -> AnkiRecoveryObservation {
    AnkiRecoveryObservation::Note(AnkiReconciliation::Existing(
        AnkiExistingNotes::new(vec![i64::MAX.try_into().unwrap()]).unwrap(),
    ))
}

#[test]
fn anki_reconciliation_worker_waits_for_commit_without_blocking_local_core_actions() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage.begin_anki_submission(&submission()).unwrap();
    let router = database.router(true);
    refresh(&router);
    let command = captured(&router, existing());
    let sql = Connection::open(database.path()).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(router
        .update(Event::CommitAnkiRecovery(command.clone()), |_| {})
        .is_empty());
    assert!(router.view().2);
    assert!(!router.view().1.allows_preparation());
    let effects = router.update(Event::OpenSettings, |_| {});
    assert!(!effects.is_empty());
    assert!(router.view().0.settings_open);
    assert!(!router.view().0.storage_busy);
    assert!(router
        .update(Event::CommitAnkiRecovery(command), |_| {})
        .is_empty());
    sql.execute_batch("ROLLBACK").unwrap();
    assert!(refresh(&router).1.allows_preparation());
    assert!(!router.view().2);
    assert_eq!(router.view().3, None);
    assert!(storage.anki_recovery().unwrap().is_none());
    let restarted = database.router(true);
    assert!(refresh(&restarted).1.allows_preparation());
}

#[test]
fn anki_reconciliation_worker_commit_failures_preserve_obligation_until_read_only_retry() {
    for model in [false, true] {
        for deferred in [false, true] {
            let database = Database::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
            if model {
                storage.begin_anki_model_creation(&creation).unwrap();
            } else {
                storage.begin_anki_submission(&submission()).unwrap();
            }
            let router = database.router(true);
            let before = refresh(&router).1;
            let sql = Connection::open(database.path()).unwrap();
            sql.execute_batch("CREATE TABLE deferred_reconcile (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
            let action = if deferred {
                "INSERT INTO deferred_reconcile VALUES (-1);"
            } else {
                "SELECT RAISE(ABORT, 'fixture');"
            };
            let table = if model {
                "anki_model_creations"
            } else {
                "anki_submissions"
            };
            sql.execute_batch(&format!(
                "CREATE TRIGGER fail_reconcile AFTER UPDATE ON {table} BEGIN {action} END;"
            ))
            .unwrap();
            let observation = if model {
                AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
                    id: 1.try_into().unwrap(),
                    layout: Box::new(creation.layout().clone()),
                })
            } else {
                existing()
            };
            let command = captured(&router, observation);
            router.update(Event::CommitAnkiRecovery(command.clone()), |_| {});
            assert_eq!(refresh(&router).1, before);
            assert_eq!(router.view().3, Some(DocumentError::Unavailable));
            assert_eq!(router.view().0.document_error, None);
            assert!(!router.view().2);
            sql.execute_batch("DROP TRIGGER fail_reconcile").unwrap();
            // Same observation is not replayed after failure, even if DB works.
            router.update(Event::CommitAnkiRecovery(command), |_| {});
            assert_eq!(refresh(&router).1, before);
            router.update(Event::RetryAnkiRecovery, |_| {});
            assert_eq!(refresh(&router).1, before);
            assert_eq!(router.view().3, None);
            assert_eq!(
                database.router(true).view().1,
                AnkiRecoveryState::Unrestored
            );
        }
    }
}

#[test]
fn anki_reconciliation_lost_commit_response_restores_terminal_state_without_replay() {
    for model in [false, true] {
        let database = Database::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
        if model {
            storage.begin_anki_model_creation(&creation).unwrap();
        } else {
            storage.begin_anki_submission(&submission()).unwrap();
        }
        let router = database.router_with_loss(true, true);
        let before = refresh(&router).1;
        let observation = if model {
            AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
                id: 1.try_into().unwrap(),
                layout: Box::new(creation.layout().clone()),
            })
        } else {
            existing()
        };
        router.update(
            Event::CommitAnkiRecovery(captured(&router, observation)),
            |_| {},
        );
        assert_eq!(refresh(&router).1, before);
        assert_eq!(router.view().3, Some(DocumentError::Unavailable));
        assert_eq!(
            storage.anki_recovery_snapshot().unwrap(),
            Default::default()
        );
        router.update(Event::RetryAnkiRecovery, |_| {});
        assert!(refresh(&router).1.allows_preparation());
        assert_eq!(router.view().3, None);
        let restarted = database.router(true);
        assert!(refresh(&restarted).1.allows_preparation());
    }
}

#[test]
fn anki_reconciliation_external_newer_journal_wins_over_stale_worker_observation() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage.begin_anki_submission(&submission()).unwrap();
    let router = database.router(true);
    refresh(&router);
    let stale = captured(&router, existing());
    storage
        .finish_anki_submission(&live, AnkiSubmissionOutcome::NotInserted)
        .unwrap();
    let creation = AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap();
    let newer = storage.begin_anki_model_creation(&creation).unwrap();
    router.update(Event::CommitAnkiRecovery(stale), |_| {});
    assert!(!refresh(&router).1.allows_preparation());
    assert_eq!(router.view().3, Some(DocumentError::Unavailable));
    assert_eq!(
        storage.anki_model_recovery().unwrap().unwrap().id,
        newer.id()
    );
    router.update(Event::RetryAnkiRecovery, |_| {});
    let AnkiRecoveryState::Restored(snapshot) = refresh(&router).1 else {
        panic!("restored")
    };
    assert!(snapshot.note.is_none());
    assert_eq!(snapshot.model.as_ref().unwrap().id, newer.id());
}
