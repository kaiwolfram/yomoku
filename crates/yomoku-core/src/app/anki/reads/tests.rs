use super::*;
use crate::{
    anki::{
        AnkiExistingNotes, AnkiModelCreation, AnkiModelLayout, AnkiModelReconciliation,
        AnkiModelRecovery, AnkiNote, AnkiReconciliation, AnkiRecovery, AnkiRecoverySnapshot,
        AnkiSubmission,
    },
    document::{TextOffset, TextRange},
    vocabulary::Surface,
};

fn fixture() -> Model {
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* old frozen 𠮷 */");
    let note = AnkiNote::new(
        &Surface::new("𠮷猫").unwrap(),
        "ねこ",
        &["cat"],
        "𠮷猫。",
        TextRange {
            start: TextOffset::START,
            end: 7.try_into().unwrap(),
        },
    )
    .unwrap();
    let snapshot = AnkiRecoverySnapshot {
        note: Some(AnkiRecovery {
            id: i64::MAX.try_into().unwrap(),
            submission: AnkiSubmission::new(
                note,
                9007199254740993_i64.try_into().unwrap(),
                i64::MAX.try_into().unwrap(),
                layout.clone(),
            )
            .unwrap(),
            known_note_id: Some(i64::MAX.try_into().unwrap()),
        }),
        model: Some(AnkiModelRecovery {
            id: i64::MAX.try_into().unwrap(),
            creation: AnkiModelCreation::new(layout).unwrap(),
            known_model_id: Some(9007199254740993_i64.try_into().unwrap()),
        }),
    };
    Model {
        anki_recovery: AnkiRecoveryState::Restored(Box::new(snapshot)),
        ..Model::default()
    }
}

fn no_effect(model: &mut Model, event: Event) {
    assert!(Yomoku.update(event, model).effects().next().is_none());
}

fn begin(model: &mut Model, target: AnkiRecoveryReadTarget) -> AnkiRecoveryReadRequest {
    no_effect(model, Event::BeginAnkiRecoveryRead(target));
    model.pending_anki_recovery_read().unwrap().clone()
}

fn existing() -> AnkiRecoveryObservation {
    AnkiRecoveryObservation::Note(AnkiReconciliation::Existing(
        AnkiExistingNotes::new(vec![i64::MAX.try_into().unwrap()]).unwrap(),
    ))
}

fn callback(request: &AnkiRecoveryReadRequest, observation: AnkiRecoveryObservation) -> Event {
    Event::AnkiRecoveryReadFinished(Box::new(request.clone()), Box::new(observation))
}

fn commit(
    model: &mut Model,
    request: &AnkiRecoveryReadRequest,
    observation: AnkiRecoveryObservation,
) -> Command<Effect, Event> {
    let mut command = Yomoku.update(callback(request, observation.clone()), model);
    let effects: Vec<_> = command.effects().collect();
    assert_eq!(effects.len(), 1);
    assert!(matches!(&effects[0], Effect::Storage(effect)
    if effect.operation == StorageOperation::CommitAnkiRecovery(Box::new(AnkiRecoveryCommit {
        expected: Box::new(request.expected().clone()), observation,
    }))));
    assert!(model.pending_anki_recovery_read().is_none());
    assert!(model.anki_recovery_saving());
    command
}

#[test]
fn anki_reads_require_restored_target_without_pending_or_failed_commit() {
    let target = AnkiRecoveryReadTarget::Note;
    let mut model = fixture();
    let restored = model.anki_recovery.clone();
    for state in [
        AnkiRecoveryState::Unrestored,
        AnkiRecoveryState::Loading,
        AnkiRecoveryState::Failed(DocumentError::Unavailable),
        AnkiRecoveryState::Restored(Box::default()),
    ] {
        model.anki_recovery = state;
        no_effect(&mut model, Event::BeginAnkiRecoveryRead(target));
        assert!(model.pending_anki_recovery_read().is_none());
    }
    model.anki_recovery = restored;
    model.anki_recovery_commit_error = Some(DocumentError::Unavailable);
    no_effect(&mut model, Event::BeginAnkiRecoveryRead(target));
    assert!(model.pending_anki_recovery_read().is_none());
    model.anki_recovery_commit_error = None;
    let AnkiRecoveryState::Restored(snapshot) = &model.anki_recovery else {
        unreachable!()
    };
    let payload = Box::new(AnkiRecoveryCommit {
        expected: snapshot.clone(),
        observation: existing(),
    });
    let mut command = Yomoku.update(Event::CommitAnkiRecovery(payload), &mut model);
    assert!(command.effects().next().is_some());
    no_effect(&mut model, Event::BeginAnkiRecoveryRead(target));
    assert!(model.pending_anki_recovery_read().is_none());
}

#[test]
fn anki_reads_freeze_full_pair_coalesce_and_clear_cannot_drop_handle() {
    let mut model = fixture();
    model.storage_busy = true;
    model.document_error = Some(DocumentError::MissingAnalysis);
    let before = Yomoku.view(&model);
    let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
    let note = request.expected().note.as_ref().unwrap();
    assert_eq!(note.submission.note().surface().as_str(), "𠮷猫");
    assert_eq!(i64::from(note.submission.deck()), 9007199254740993);
    assert_eq!(i64::from(note.known_note_id.unwrap()), i64::MAX);
    assert!(request
        .expected()
        .model
        .as_ref()
        .unwrap()
        .creation
        .layout()
        .css
        .ends_with("/* old frozen 𠮷 */"));
    for event in [
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Note),
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
        Event::RestoreAnkiRecovery,
        Event::ClearPreparedAnkiExport,
        Event::CommitAnkiRecovery(Box::new(AnkiRecoveryCommit {
            expected: Box::new(request.expected().clone()),
            observation: existing(),
        })),
    ] {
        no_effect(&mut model, event);
        assert_eq!(model.pending_anki_recovery_read(), Some(&request));
    }
    assert_eq!(Yomoku.view(&model), before);
    assert!(!model.anki_recovery_saving());
}

#[test]
fn anki_reads_explicit_retry_invalidates_lost_read_before_restore_or_exhaustion() {
    for exhausted in [false, true] {
        let mut model = fixture();
        let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
        if exhausted {
            model.anki_recovery_sequence = u64::MAX;
        }
        let mut retry = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
        assert!(model.pending_anki_recovery_read().is_none());
        no_effect(&mut model, callback(&request, existing()));
        if exhausted {
            assert!(retry.effects().next().is_none());
            assert_eq!(
                model.anki_recovery,
                AnkiRecoveryState::Failed(DocumentError::Unavailable)
            );
            continue;
        }
        assert!(
            matches!(retry.effects().next(), Some(Effect::Storage(request)) if request.operation == StorageOperation::LoadAnkiRecovery)
        );
        assert_eq!(model.anki_recovery, AnkiRecoveryState::Loading);
        no_effect(
            &mut model,
            Event::AnkiRecoveryFinished(
                1,
                StorageResult::AnkiRecoveryLoaded(Box::new(request.expected().clone())),
            ),
        );
        let fresh = begin(&mut model, AnkiRecoveryReadTarget::Note);
        no_effect(&mut model, callback(&request, existing()));
        assert_eq!(model.pending_anki_recovery_read(), Some(&fresh));
        assert_ne!(fresh, request);
    }
}

#[test]
fn anki_reads_reject_other_owners_duplicate_callbacks_and_identical_later_attempts() {
    let mut model = fixture();
    let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
    let mut other = fixture();
    let foreign = begin(&mut other, AnkiRecoveryReadTarget::Note);
    assert_eq!(request.expected(), foreign.expected());
    assert_ne!(request, foreign);
    no_effect(&mut model, callback(&foreign, existing()));
    assert_eq!(model.pending_anki_recovery_read(), Some(&request));
    let unresolved = AnkiRecoveryObservation::Note(AnkiReconciliation::Unresolved);
    let _command = commit(&mut model, &request, unresolved);
    no_effect(&mut model, callback(&request, existing()));
    no_effect(
        &mut model,
        Event::AnkiRecoveryCommitFinished(
            1,
            StorageResult::AnkiRecoveryCommitted(Box::new(request.expected().clone())),
        ),
    );
    let later = begin(&mut model, AnkiRecoveryReadTarget::Note);
    assert_ne!(later, request);
    no_effect(&mut model, callback(&request, existing()));
    assert_eq!(model.pending_anki_recovery_read(), Some(&later));
    assert!(Model::default().pending_anki_recovery_read().is_none());
}

#[test]
fn anki_reads_wrong_target_or_changed_pair_cannot_be_rebound_to_current_snapshot() {
    for changed in [false, true] {
        let mut model = fixture();
        let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
        let observation = if changed {
            let AnkiRecoveryState::Restored(snapshot) = &mut model.anki_recovery else {
                unreachable!()
            };
            snapshot.model.as_mut().unwrap().known_model_id = Some(1.try_into().unwrap());
            existing()
        } else {
            AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved)
        };
        let before = model.anki_recovery.clone();
        no_effect(&mut model, callback(&request, observation));
        assert!(model.pending_anki_recovery_read().is_none());
        assert_eq!(
            model.anki_recovery_read_error(),
            Some(AnkiExportError::InvalidResponse)
        );
        assert_eq!(model.anki_recovery, before);
        assert!(!model.anki_recovery_saving());
        let retry = begin(&mut model, AnkiRecoveryReadTarget::Note);
        assert_ne!(retry, request);
        assert_eq!(model.anki_recovery_read_error(), None);
    }
}

#[test]
fn anki_reads_success_keeps_obligation_until_commit_and_resolves_only_target() {
    for target in [AnkiRecoveryReadTarget::Note, AnkiRecoveryReadTarget::Model] {
        let mut model = fixture();
        let request = begin(&mut model, target);
        let observation = match target {
            AnkiRecoveryReadTarget::Note => existing(),
            AnkiRecoveryReadTarget::Model => {
                let recovery = request.expected().model.as_ref().unwrap();
                AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
                    id: recovery.known_model_id.unwrap(),
                    layout: Box::new(recovery.creation.layout().clone()),
                })
            }
        };
        let expected = AnkiRecoveryCommit {
            expected: Box::new(request.expected().clone()),
            observation: observation.clone(),
        }
        .expected_result()
        .unwrap();
        let _command = commit(&mut model, &request, observation);
        assert_eq!(
            model.anki_recovery,
            AnkiRecoveryState::Restored(Box::new(request.expected().clone()))
        );
        no_effect(
            &mut model,
            Event::AnkiRecoveryCommitFinished(
                1,
                StorageResult::AnkiRecoveryCommitted(expected.clone()),
            ),
        );
        assert_eq!(model.anki_recovery, AnkiRecoveryState::Restored(expected));
        assert!(!model.anki_recovery().allows_preparation());
        assert_eq!(model.anki_recovery_read_error(), None);
    }
}

#[test]
fn anki_reads_missing_failed_or_incompatible_evidence_preserves_obligations() {
    for observation in [
        AnkiRecoveryObservation::Note(AnkiReconciliation::Unresolved),
        AnkiRecoveryObservation::Note(AnkiReconciliation::Failed(
            AnkiExportError::PermissionDenied,
        )),
        AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved),
        AnkiRecoveryObservation::Model(AnkiModelReconciliation::Observed {
            id: 1.try_into().unwrap(),
            layout: Box::new(AnkiModelLayout::yomoku()),
        }),
    ] {
        let mut model = fixture();
        let target = match observation {
            AnkiRecoveryObservation::Note(_) => AnkiRecoveryReadTarget::Note,
            AnkiRecoveryObservation::Model(_) => AnkiRecoveryReadTarget::Model,
        };
        let request = begin(&mut model, target);
        let _command = commit(&mut model, &request, observation);
        no_effect(
            &mut model,
            Event::AnkiRecoveryCommitFinished(
                1,
                StorageResult::AnkiRecoveryCommitted(Box::new(request.expected().clone())),
            ),
        );
        assert_eq!(
            model.anki_recovery,
            AnkiRecoveryState::Restored(Box::new(request.expected().clone()))
        );
        assert!(!model.anki_recovery().allows_preparation());
        assert!(model.pending_anki_recovery_read().is_none());
    }
}

#[test]
fn anki_reads_failed_or_lost_commit_requires_restore_and_fresh_provider_handle() {
    let mut model = fixture();
    let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
    let _command = commit(&mut model, &request, existing());
    no_effect(
        &mut model,
        Event::AnkiRecoveryCommitFinished(1, StorageResult::Failed(DocumentError::Unavailable)),
    );
    no_effect(
        &mut model,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Note),
    );
    no_effect(&mut model, callback(&request, existing()));
    assert!(model.pending_anki_recovery_read().is_none());
    let mut retry = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
    assert!(
        matches!(retry.effects().next(), Some(Effect::Storage(request)) if request.operation == StorageOperation::LoadAnkiRecovery)
    );
    no_effect(
        &mut model,
        Event::AnkiRecoveryFinished(
            2,
            StorageResult::AnkiRecoveryLoaded(Box::new(request.expected().clone())),
        ),
    );
    let fresh = begin(&mut model, AnkiRecoveryReadTarget::Note);
    assert_ne!(fresh, request);
    no_effect(&mut model, callback(&request, existing()));
    assert_eq!(model.pending_anki_recovery_read(), Some(&fresh));
}

#[test]
fn anki_reads_commit_attempt_exhaustion_never_releases_frozen_obligation() {
    let mut model = fixture();
    let request = begin(&mut model, AnkiRecoveryReadTarget::Note);
    model.anki_recovery_sequence = u64::MAX;
    no_effect(&mut model, callback(&request, existing()));
    assert!(model.pending_anki_recovery_read().is_none());
    assert!(!model.anki_recovery_saving());
    assert_eq!(
        model.anki_recovery_commit_error(),
        Some(DocumentError::Unavailable)
    );
    assert_eq!(
        model.anki_recovery,
        AnkiRecoveryState::Restored(Box::new(request.expected().clone()))
    );
}
