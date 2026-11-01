use super::*;
use crate::anki::{
    AnkiExportError, AnkiModelCreation, AnkiModelLayout, AnkiModelRecovery, AnkiRecoveryReadResult,
    AnkiRecoveryReadTarget, AnkiRecoverySnapshot,
};

fn fixture() -> Model {
    Model {
        settings_open: true,
        anki_recovery: AnkiRecoveryState::Restored(Box::new(AnkiRecoverySnapshot {
            note: None,
            model: Some(AnkiModelRecovery {
                id: 1.try_into().unwrap(),
                creation: AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                known_model_id: None,
            }),
        })),
        ..Model::default()
    }
}

fn view(model: &Model) -> AnkiRecoveryView {
    Yomoku.view(model).anki_recovery.unwrap()
}

fn effects(model: &mut Model, event: Event) -> Vec<Effect> {
    Yomoku.update(event, model).effects().collect()
}

#[test]
fn anki_view_is_settings_scoped_and_opening_never_reads_provider() {
    let mut model = fixture();
    model.settings_open = false;
    assert!(Yomoku.view(&model).anki_recovery.is_none());
    let output = effects(&mut model, Event::OpenSettings);
    assert!(output
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
    let visible = view(&model);
    assert_eq!(visible.phase, AnkiRecoveryPhase::Unresolved);
    assert!(visible.model_pending && visible.can_check_model);
    assert!(!visible.note_pending && !visible.can_check_note);
    assert!(!visible.can_retry);
}

#[test]
fn anki_view_renders_restore_failure_and_explicit_retry_without_reader_changes() {
    let mut model = Model {
        settings_open: true,
        storage_busy: true,
        document_error: Some(DocumentError::MissingAnalysis),
        ..Model::default()
    };
    assert!(Yomoku.view(&model).anki_recovery.is_none());
    effects(&mut model, Event::RestoreAnkiRecovery);
    let output = effects(
        &mut model,
        Event::AnkiRecoveryFinished(1, StorageResult::Failed(DocumentError::Unavailable)),
    );
    assert!(matches!(output.as_slice(), [Effect::Render(_)]));
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Failed);
    assert_eq!(view(&model).storage_error, Some(DocumentError::Unavailable));
    assert!(view(&model).can_retry);
    let output = effects(&mut model, Event::RetryAnkiRecovery);
    assert!(output
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(_))));
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Loading);
    assert!(!view(&model).can_retry);
    effects(
        &mut model,
        Event::AnkiRecoveryFinished(2, StorageResult::AnkiRecoveryLoaded(Box::default())),
    );
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Ready);
    assert!(model.storage_busy);
    assert_eq!(model.document_error, Some(DocumentError::MissingAnalysis));
}

#[test]
fn anki_view_keeps_empty_observation_unresolved_until_and_after_commit() {
    let mut model = fixture();
    effects(
        &mut model,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
    );
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Checking);
    assert!(view(&model).can_retry);
    assert!(!view(&model).can_check_model);
    let request = model.anki_recovery_read.clone().unwrap();
    let output = effects(
        &mut model,
        Event::AnkiRecoveryProviderFinished(Box::new(request), AnkiRecoveryReadResult::Model(None)),
    );
    assert!(output
        .iter()
        .any(|effect| matches!(effect, Effect::Render(_))));
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Saving);
    assert!(view(&model).model_pending);
    assert!(!view(&model).can_retry && !view(&model).can_check_model);
    let snapshot = match &model.anki_recovery {
        AnkiRecoveryState::Restored(pair) => pair.clone(),
        _ => unreachable!(),
    };
    let id = model.anki_recovery_commit.as_ref().unwrap().0;
    effects(
        &mut model,
        Event::AnkiRecoveryCommitFinished(id, StorageResult::AnkiRecoveryCommitted(snapshot)),
    );
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Unresolved);
    assert!(view(&model).can_check_model);
}

#[test]
fn anki_view_exposes_provider_error_and_lost_read_retry_rejects_late_callback() {
    let mut model = fixture();
    effects(
        &mut model,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
    );
    let request = model.anki_recovery_read.clone().unwrap();
    effects(&mut model, Event::RetryAnkiRecovery);
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Loading);
    assert!(effects(
        &mut model,
        Event::AnkiRecoveryProviderFinished(
            Box::new(request),
            AnkiRecoveryReadResult::Model(Some(9))
        )
    )
    .is_empty());
    let snapshot = fixture().anki_recovery;
    let AnkiRecoveryState::Restored(pair) = snapshot else {
        unreachable!()
    };
    effects(
        &mut model,
        Event::AnkiRecoveryFinished(1, StorageResult::AnkiRecoveryLoaded(pair)),
    );
    effects(
        &mut model,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
    );
    let request = model.anki_recovery_read.clone().unwrap();
    effects(
        &mut model,
        Event::AnkiRecoveryProviderFinished(
            Box::new(request),
            AnkiRecoveryReadResult::Failed(AnkiExportError::PermissionDenied),
        ),
    );
    assert_eq!(
        view(&model).provider_error,
        Some(AnkiExportError::PermissionDenied)
    );
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Saving);
    let id = model.anki_recovery_commit.as_ref().unwrap().0;
    effects(
        &mut model,
        Event::AnkiRecoveryCommitFinished(id, StorageResult::Failed(DocumentError::Unavailable)),
    );
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Failed);
    assert!(view(&model).can_retry);
    assert!(!view(&model).can_check_model);
}

#[test]
fn anki_view_success_requires_durable_acknowledgement_and_ignores_duplicate() {
    let mut model = fixture();
    effects(
        &mut model,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
    );
    let request = model.anki_recovery_read.clone().unwrap();
    effects(
        &mut model,
        Event::AnkiRecoveryProviderFinished(
            Box::new(request),
            AnkiRecoveryReadResult::Model(Some(i64::MAX)),
        ),
    );
    assert!(view(&model).model_pending);
    let id = model.anki_recovery_commit.as_ref().unwrap().0;
    let output = effects(
        &mut model,
        Event::AnkiRecoveryCommitFinished(id, StorageResult::AnkiRecoveryCommitted(Box::default())),
    );
    assert!(matches!(output.as_slice(), [Effect::Render(_)]));
    assert_eq!(view(&model).phase, AnkiRecoveryPhase::Ready);
    assert!(!view(&model).model_pending);
    assert!(effects(
        &mut model,
        Event::AnkiRecoveryCommitFinished(id, StorageResult::Failed(DocumentError::Unavailable))
    )
    .is_empty());
}

#[test]
fn anki_view_live_work_and_failed_commit_never_offer_recovery_reads() {
    use crate::anki::{AnkiLiveIntent, AnkiLiveRequest, AnkiLiveState};
    let mut model = fixture();
    for progress in [
        AnkiLiveProgress::Beginning,
        AnkiLiveProgress::Ready,
        AnkiLiveProgress::Invoking,
        AnkiLiveProgress::Saving,
        AnkiLiveProgress::Failed(DocumentError::Unavailable),
        AnkiLiveProgress::Finished,
    ] {
        model.anki_live = Some(AnkiLiveState {
            request: AnkiLiveRequest::new(AnkiLiveIntent::Model(Box::new(
                AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
            ))),
            progress,
            begun: None,
            callback: None,
        });
        assert_eq!(
            view(&model).can_check_model,
            progress == AnkiLiveProgress::Finished
        );
        assert_eq!(
            view(&model).can_retry,
            matches!(progress, AnkiLiveProgress::Failed(_))
        );
        let expected = match progress {
            AnkiLiveProgress::Finished => AnkiRecoveryPhase::Unresolved,
            AnkiLiveProgress::Failed(_) => AnkiRecoveryPhase::Failed,
            _ => AnkiRecoveryPhase::Busy,
        };
        assert_eq!(view(&model).phase, expected);
    }
}

#[test]
fn anki_view_empty_startup_and_fresh_session_show_no_optional_integration() {
    let mut model = Model {
        settings_open: true,
        ..Model::default()
    };
    assert!(Yomoku.view(&model).anki_recovery.is_none());
    effects(&mut model, Event::RestoreAnkiRecovery);
    assert!(Yomoku.view(&model).anki_recovery.is_none());
    let output = effects(
        &mut model,
        Event::AnkiRecoveryFinished(1, StorageResult::AnkiRecoveryLoaded(Box::default())),
    );
    assert!(output.is_empty());
    assert!(Yomoku.view(&model).anki_recovery.is_none());
}
