use super::*;
use crate::anki::{
    AnkiExistingNotes, AnkiModelCreation, AnkiModelCreationId, AnkiModelId, AnkiModelLayout,
    AnkiModelReconciliation, AnkiModelRecovery, AnkiReconciliation, AnkiRecoveryCommit,
    AnkiRecoveryObservation, AnkiRecoverySnapshot,
};

fn fixture() -> (Model, Box<AnkiRecoveryCommit>) {
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* frozen 𠮷 */");
    let snapshot = Box::new(AnkiRecoverySnapshot {
        note: None,
        model: Some(AnkiModelRecovery {
            id: AnkiModelCreationId::try_from(i64::MAX).unwrap(),
            creation: AnkiModelCreation::new(layout.clone()).unwrap(),
            known_model_id: Some(AnkiModelId::try_from(9007199254740993).unwrap()),
        }),
    });
    let model = Model {
        anki_recovery: AnkiRecoveryState::Restored(snapshot.clone()),
        ..Model::default()
    };
    let command = Box::new(AnkiRecoveryCommit {
        expected: snapshot,
        observation: AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
            id: AnkiModelId::try_from(9007199254740993).unwrap(),
            layout: Box::new(layout),
        }),
    });
    (model, command)
}

fn dispatch(model: &mut Model, payload: Box<AnkiRecoveryCommit>) -> Command<Effect, Event> {
    let mut command = Yomoku.update(Event::CommitAnkiRecovery(payload.clone()), model);
    let effects: Vec<_> = command.effects().collect();
    assert_eq!(effects.len(), 1);
    assert!(
        matches!(&effects[0], Effect::Storage(request) if request.operation == StorageOperation::CommitAnkiRecovery(payload))
    );
    command
}

fn finish(model: &mut Model, id: u64, result: StorageResult) {
    assert!(Yomoku
        .update(Event::AnkiRecoveryCommitFinished(id, result), model)
        .effects()
        .next()
        .is_none());
}

#[test]
fn anki_reconciliation_requires_exact_restored_target_and_no_failed_commit() {
    let (mut model, payload) = fixture();
    let restored = model.anki_recovery.clone();
    for state in [
        AnkiRecoveryState::Unrestored,
        AnkiRecoveryState::Loading,
        AnkiRecoveryState::Failed(DocumentError::Unavailable),
        AnkiRecoveryState::Restored(Box::default()),
    ] {
        model.anki_recovery = state;
        assert!(Yomoku
            .update(Event::CommitAnkiRecovery(payload.clone()), &mut model)
            .effects()
            .next()
            .is_none());
    }
    model.anki_recovery = restored;
    let mut changed = payload.clone();
    changed.expected.model.as_mut().unwrap().known_model_id = None;
    assert!(Yomoku
        .update(Event::CommitAnkiRecovery(changed), &mut model)
        .effects()
        .next()
        .is_none());
    let mut wrong_kind = payload.clone();
    wrong_kind.observation = AnkiRecoveryObservation::Note(AnkiReconciliation::Unresolved);
    assert!(Yomoku
        .update(Event::CommitAnkiRecovery(wrong_kind), &mut model)
        .effects()
        .next()
        .is_none());
    model.anki_recovery_commit_error = Some(DocumentError::Unavailable);
    assert!(Yomoku
        .update(Event::CommitAnkiRecovery(payload), &mut model)
        .effects()
        .next()
        .is_none());
}

#[test]
fn anki_reconciliation_opens_gate_only_after_exact_commit_callback_and_preserves_reader() {
    let (mut model, payload) = fixture();
    model.storage_busy = true;
    model.document_error = Some(DocumentError::MissingAnalysis);
    let before = Yomoku.view(&model);
    let mut command = Yomoku.update(Event::CommitAnkiRecovery(payload.clone()), &mut model);
    assert!(model.anki_recovery_saving());
    assert!(!model.anki_recovery().allows_preparation());
    for event in [
        Event::CommitAnkiRecovery(payload),
        Event::RetryAnkiRecovery,
        Event::RestoreAnkiRecovery,
    ] {
        assert!(Yomoku.update(event, &mut model).effects().next().is_none());
    }
    finish(
        &mut model,
        0,
        StorageResult::AnkiRecoveryCommitted(Box::default()),
    );
    assert!(model.anki_recovery_saving());
    let Effect::Storage(mut request) = command.effects().next().unwrap() else {
        panic!("storage")
    };
    request
        .resolve(StorageResult::AnkiRecoveryCommitted(Box::default()))
        .unwrap();
    let event = command.events().next().unwrap();
    assert!(matches!(event, Event::AnkiRecoveryCommitFinished(1, _)));
    assert!(Yomoku.update(event, &mut model).effects().next().is_none());
    assert!(model.anki_recovery().allows_preparation());
    assert!(!model.anki_recovery_saving());
    finish(
        &mut model,
        1,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(model.anki_recovery_commit_error(), None);
    assert_eq!(Yomoku.view(&model), before);
}

#[test]
fn anki_reconciliation_failed_or_wrong_commit_requires_read_only_restore_not_replay() {
    for result in [
        StorageResult::Failed(DocumentError::Unavailable),
        StorageResult::AnkiRecoveryLoaded(Box::default()),
        StorageResult::ImportCancelled,
    ] {
        let (mut model, payload) = fixture();
        let before = model.anki_recovery.clone();
        let _command = dispatch(&mut model, payload.clone());
        finish(&mut model, 1, result);
        assert_eq!(model.anki_recovery, before);
        assert_eq!(
            model.anki_recovery_commit_error(),
            Some(DocumentError::Unavailable)
        );
        assert!(Yomoku
            .update(Event::CommitAnkiRecovery(payload), &mut model)
            .effects()
            .next()
            .is_none());
        assert!(Yomoku
            .update(Event::RestoreAnkiRecovery, &mut model)
            .effects()
            .next()
            .is_none());
        let mut retry = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
        assert!(
            matches!(retry.effects().next(), Some(Effect::Storage(request)) if request.operation == StorageOperation::LoadAnkiRecovery)
        );
        assert_eq!(model.anki_recovery, AnkiRecoveryState::Loading);
        finish(
            &mut model,
            1,
            StorageResult::AnkiRecoveryCommitted(Box::default()),
        );
        assert_eq!(model.anki_recovery, AnkiRecoveryState::Loading);
        // A fresh authoritative restore resolves a committed-but-lost response.
        assert!(Yomoku
            .update(
                Event::AnkiRecoveryFinished(2, StorageResult::AnkiRecoveryLoaded(Box::default())),
                &mut model,
            )
            .effects()
            .next()
            .is_none());
        assert!(model.anki_recovery().allows_preparation());
    }
}

#[test]
fn anki_reconciliation_empty_or_wrong_model_cannot_accept_a_false_empty_gate() {
    for observation in [
        AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved),
        AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
            id: AnkiModelId::try_from(1).unwrap(),
            layout: Box::new(AnkiModelLayout::yomoku()),
        }),
    ] {
        let (mut model, mut payload) = fixture();
        payload.observation = observation;
        let _command = dispatch(&mut model, payload);
        finish(
            &mut model,
            1,
            StorageResult::AnkiRecoveryCommitted(Box::default()),
        );
        assert!(!model.anki_recovery().allows_preparation());
        assert!(model.anki_recovery_commit_error().is_some());
    }
}

#[test]
fn anki_reconciliation_old_callback_cannot_release_newer_commit_and_exhaustion_stays_closed() {
    let (mut model, payload) = fixture();
    let mut unresolved = payload.clone();
    unresolved.observation = AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved);
    let _first = dispatch(&mut model, unresolved.clone());
    finish(
        &mut model,
        1,
        StorageResult::AnkiRecoveryCommitted(unresolved.expected.clone()),
    );
    assert!(!model.anki_recovery().allows_preparation());
    let _second = dispatch(&mut model, payload);
    finish(
        &mut model,
        1,
        StorageResult::AnkiRecoveryCommitted(Box::default()),
    );
    assert!(model.anki_recovery_saving());
    assert!(!model.anki_recovery().allows_preparation());
    finish(
        &mut model,
        2,
        StorageResult::AnkiRecoveryCommitted(Box::default()),
    );
    assert!(model.anki_recovery().allows_preparation());
    let (mut model, payload) = fixture();
    model.anki_recovery_sequence = u64::MAX;
    assert!(Yomoku
        .update(Event::CommitAnkiRecovery(payload), &mut model)
        .effects()
        .next()
        .is_none());
    assert!(model.anki_recovery_commit_error().is_some());
    assert!(!model.anki_recovery().allows_preparation());
}

#[test]
fn anki_reconciliation_one_resolved_slot_cannot_hide_another_obligation() {
    let (mut model, mut payload) = fixture();
    // A pair with both obligations is conservative even if normal preparation
    // gates prevent this situation. The model observation can release only one.
    let note = crate::anki::AnkiNote::new(
        &crate::vocabulary::Surface::new("猫").unwrap(),
        "",
        &[],
        "猫",
        crate::document::TextRange {
            start: crate::document::TextOffset::START,
            end: 3.try_into().unwrap(),
        },
    )
    .unwrap();
    payload.expected.note = Some(crate::anki::AnkiRecovery {
        id: 1.try_into().unwrap(),
        submission: crate::anki::AnkiSubmission::new(
            note,
            1.try_into().unwrap(),
            1.try_into().unwrap(),
            AnkiModelLayout::yomoku(),
        )
        .unwrap(),
        known_note_id: None,
    });
    model.anki_recovery = AnkiRecoveryState::Restored(payload.expected.clone());
    let after_model = payload.expected_result().unwrap();
    let _command = dispatch(&mut model, payload);
    finish(
        &mut model,
        1,
        StorageResult::AnkiRecoveryCommitted(after_model.clone()),
    );
    assert!(!model.anki_recovery().allows_preparation());
    let note_commit = Box::new(AnkiRecoveryCommit {
        expected: after_model,
        observation: AnkiRecoveryObservation::Note(AnkiReconciliation::Existing(
            AnkiExistingNotes::new(vec![i64::MAX.try_into().unwrap()]).unwrap(),
        )),
    });
    let _command = dispatch(&mut model, note_commit);
    finish(
        &mut model,
        2,
        StorageResult::AnkiRecoveryCommitted(Box::default()),
    );
    assert!(model.anki_recovery().allows_preparation());
}
