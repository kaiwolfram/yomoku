use super::*;
use yomoku_core::anki::{
    AnkiExistingNotes, AnkiModelReconciliation, AnkiReconciliation, AnkiRecoveryObservation,
    AnkiRecoveryReadRequest, AnkiRecoveryReadTarget,
};

fn read(router: &Router, model: bool) -> (AnkiRecoveryReadRequest, AnkiRecoveryObservation) {
    let target = if model {
        AnkiRecoveryReadTarget::Model
    } else {
        AnkiRecoveryReadTarget::Note
    };
    assert!(router
        .update(Event::BeginAnkiRecoveryRead(target), |_| panic!("no I/O"))
        .is_empty());
    let request = router.view().4.unwrap();
    let observation = if model {
        let recovery = request.expected().model.as_ref().unwrap();
        AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
            id: 9007199254740993_i64.try_into().unwrap(),
            layout: Box::new(recovery.creation.layout().clone()),
        })
    } else {
        AnkiRecoveryObservation::Note(AnkiReconciliation::Existing(
            AnkiExistingNotes::new(vec![i64::MAX.try_into().unwrap()]).unwrap(),
        ))
    };
    (request, observation)
}

fn finish(request: &AnkiRecoveryReadRequest, observation: &AnkiRecoveryObservation) -> Event {
    Event::AnkiRecoveryReadFinished(Box::new(request.clone()), Box::new(observation.clone()))
}

fn prepare(storage: &mut Storage, model: bool) {
    if model {
        let creation = AnkiModelCreation::new(submission().layout().clone()).unwrap();
        storage.begin_anki_model_creation(&creation).unwrap();
    } else {
        storage.begin_anki_submission(&submission()).unwrap();
    }
}

#[test]
fn anki_reads_worker_commits_captured_pair_and_lost_result_recovers_without_replay() {
    for model in [false, true] {
        for lost in [false, true] {
            let database = Database::new();
            let mut storage = Storage::open(database.path()).unwrap();
            prepare(&mut storage, model);
            let router = database.router_with_loss(true, lost);
            let before = refresh(&router).1;
            let bytes = std::fs::read(database.path()).unwrap();
            let (request, observation) = read(&router, model);
            assert_eq!(std::fs::read(database.path()).unwrap(), bytes);
            assert_eq!(refresh(&router).1, before);
            assert_eq!(router.view().4.as_ref(), Some(&request));
            let restarted = database.router(true);
            refresh(&restarted);
            let (other, _) = read(&restarted, model);
            assert_ne!(request, other);
            router.update(finish(&other, &observation), |_| panic!("foreign callback"));
            assert_eq!(router.view().4.as_ref(), Some(&request));
            router.update(finish(&request, &observation), |_| {});
            let after = refresh(&router).1;
            assert_eq!(
                storage.anki_recovery_snapshot().unwrap(),
                Default::default()
            );
            assert!(router.view().4.is_none());
            if lost {
                assert_eq!(after, before);
                assert_eq!(router.view().3, Some(DocumentError::Unavailable));
                router.update(finish(&request, &observation), |_| panic!("no replay"));
                router.update(Event::RetryAnkiRecovery, |_| {});
                assert!(refresh(&router).1.allows_preparation());
            } else {
                assert!(after.allows_preparation());
            }
            router.update(finish(&request, &observation), |_| panic!("duplicate"));
            assert!(refresh(&router).1.allows_preparation());
        }
    }
}

#[test]
fn anki_reads_worker_failure_requires_restore_then_new_read() {
    for model in [false, true] {
        let database = Database::new();
        let mut storage = Storage::open(database.path()).unwrap();
        prepare(&mut storage, model);
        let router = database.router(true);
        let before = refresh(&router).1;
        let (request, observation) = read(&router, model);
        let sql = Connection::open(database.path()).unwrap();
        let table = if model {
            "anki_model_creations"
        } else {
            "anki_submissions"
        };
        sql.execute_batch(&format!(
            "CREATE TRIGGER fail_read_commit AFTER UPDATE ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
        )).unwrap();
        router.update(finish(&request, &observation), |_| {});
        assert_eq!(refresh(&router).1, before);
        assert_eq!(router.view().3, Some(DocumentError::Unavailable));
        sql.execute_batch("DROP TRIGGER fail_read_commit").unwrap();
        router.update(finish(&request, &observation), |_| panic!("no replay"));
        assert_eq!(refresh(&router).1, before);
        router.update(
            Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Note),
            |_| panic!("failed gate"),
        );
        assert!(router.view().4.is_none());
        router.update(Event::RetryAnkiRecovery, |_| {});
        assert_eq!(refresh(&router).1, before);
        let (fresh, observation) = read(&router, model);
        assert_ne!(fresh, request);
        router.update(finish(&request, &observation), |_| panic!("stale read"));
        assert_eq!(router.view().4.as_ref(), Some(&fresh));
        router.update(finish(&fresh, &observation), |_| {});
        assert!(refresh(&router).1.allows_preparation());
        assert_eq!(
            storage.anki_recovery_snapshot().unwrap(),
            Default::default()
        );
    }
}

#[test]
fn anki_reads_worker_rejects_external_pair_change_after_read_started() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let live = storage.begin_anki_submission(&submission()).unwrap();
    let router = database.router(true);
    let before = refresh(&router).1;
    let (request, observation) = read(&router, false);
    storage
        .finish_anki_submission(&live, AnkiSubmissionOutcome::NotInserted)
        .unwrap();
    prepare(&mut storage, true);
    let changed = storage.anki_recovery_snapshot().unwrap();
    router.update(finish(&request, &observation), |_| {});
    assert_eq!(refresh(&router).1, before);
    assert_eq!(router.view().3, Some(DocumentError::Unavailable));
    assert_eq!(storage.anki_recovery_snapshot().unwrap(), changed);
    router.update(Event::RetryAnkiRecovery, |_| {});
    assert_eq!(
        refresh(&router).1,
        AnkiRecoveryState::Restored(Box::new(changed))
    );
    assert!(router.view().4.is_none());
}
