use super::*;
use crate::anki::{AnkiExportPhase as Phase, AnkiExportView, AnkiLiveResult};

fn view(model: &Model) -> AnkiExportView {
    Yomoku.view(model).anki_export.unwrap()
}

fn effects(model: &mut Model, event: Event) -> Vec<Effect> {
    Yomoku.update(event, model).effects().collect()
}

fn assert_render(effects: &[Effect]) {
    assert!(matches!(effects, [Effect::Render(_)]));
}

fn outcome_to_save(model: &mut Model, result: AnkiLiveResult) -> AnkiLiveCallback {
    let mut command = Yomoku.update(Event::DispatchAnkiLive, model);
    let mut output = command.effects();
    let Some(Effect::AnkiLive(mut effect)) = output.next() else {
        panic!("provider dispatch")
    };
    assert!(output.all(|effect| matches!(effect, Effect::Render(_))));
    drop(output);
    assert_eq!(view(model).phase, Phase::Invoking);
    effect.resolve(result).unwrap();
    let event = command.events().next().unwrap();
    let Event::FinishAnkiLive(callback) = &event else {
        panic!("owned provider response")
    };
    let callback = *callback.clone();
    let output = effects(model, event);
    assert!(output
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(_))));
    assert!(output
        .iter()
        .any(|effect| matches!(effect, Effect::Render(_))));
    assert_eq!(view(model).phase, Phase::Saving);
    callback
}

#[test]
fn anki_export_view_freezes_surface_and_deck_renders_once_and_survives_navigation() {
    for effective in [false, true] {
        let (mut model, selection) = fixture(effective);
        let before = reader_view(&model);
        assert!(Yomoku.view(&model).anki_export.is_none());
        let event = Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        };
        assert_render(&effects(&mut model, event.clone()));
        assert_eq!(reader_view(&model), before);
        let expected = AnkiExportView {
            preparation_id: 1,
            surface: "猫".into(),
            deck_id: i64::MAX,
            phase: Phase::Prepared,
        };
        assert_eq!(view(&model), expected);
        assert!(effects(&mut model, event).is_empty());
        effects(&mut model, Event::DismissReaderWord);
        effects(&mut model, Event::CloseDocument);
        effects(&mut model, Event::OpenSettings);
        assert_eq!(view(&model), expected);
        assert_render(&effects(&mut model, Event::ClearPreparedAnkiExport));
        assert!(Yomoku.view(&model).anki_export.is_none());
    }
}

#[test]
fn anki_export_view_preflight_projects_complete_evidence_conflicts_and_provider_errors() {
    for effective in [false, true] {
        for (models, duplicates, expected) in [
            (vec![], vec![], Phase::CreateModel),
            (vec![true], vec![], Phase::ReuseModel(9007199254740993)),
            (
                vec![false],
                vec![],
                Phase::ProviderFailed(AnkiExportError::IncompatibleModel),
            ),
            (
                vec![true, true],
                vec![],
                Phase::ProviderFailed(AnkiExportError::IncompatibleModel),
            ),
            (
                vec![],
                vec![i64::MAX, 9007199254740993],
                Phase::Existing(vec![i64::MAX, 9007199254740993]),
            ),
        ] {
            let (mut model, selection) = fixture(effective);
            effects(
                &mut model,
                Event::PrepareAnkiExport {
                    selection,
                    deck: i64::MAX.try_into().unwrap(),
                },
            );
            let output = effects(&mut model, Event::BeginAnkiExportPreflight);
            assert!(output
                .iter()
                .any(|effect| matches!(effect, Effect::AnkiExportPreflight(_))));
            assert!(output
                .iter()
                .any(|effect| matches!(effect, Effect::Render(_))));
            assert_eq!(view(&model).phase, Phase::Checking);
            let request = model.pending_anki_export_preflight().unwrap().clone();
            let event = Event::AnkiExportPreflightFinished(
                Box::new(request),
                Ok(AnkiPreflightObservation {
                    decks: vec![AnkiPreflightDeck {
                        id: i64::MAX,
                        filtered: false,
                    }],
                    duplicate_note_ids: duplicates,
                    models: models
                        .into_iter()
                        .enumerate()
                        .map(|(index, matches_layout)| AnkiPreflightModel {
                            id: 9007199254740993 + index as i64,
                            name: crate::anki::MODEL_NAME.into(),
                            matches_layout,
                        })
                        .collect(),
                }),
            );
            assert_render(&effects(&mut model, event.clone()));
            assert_eq!(view(&model).phase, expected);
            assert!(effects(&mut model, event).is_empty());
        }
        for error in [
            AnkiExportError::ProviderUnavailable,
            AnkiExportError::PermissionDenied,
            AnkiExportError::InvalidResponse,
        ] {
            let mut model = prepared(effective, false);
            effects(&mut model, Event::RetryAnkiExportPreflight);
            assert_eq!(view(&model).phase, Phase::Checking);
            let request = model.pending_anki_export_preflight().unwrap().clone();
            assert_render(&effects(
                &mut model,
                Event::AnkiExportPreflightFinished(Box::new(request), Err(error)),
            ));
            assert_eq!(view(&model).phase, Phase::ProviderFailed(error));
        }
    }
}

#[test]
fn anki_export_view_exposes_terminal_results_only_after_exact_durable_acknowledgement() {
    for effective in [false, true] {
        for (create, result, expected) in [
            (false, AnkiLiveResult::NoSubmission, Phase::NotSubmitted),
            (
                false,
                AnkiLiveResult::NoteAdded(i64::MAX),
                Phase::NoteAdded(i64::MAX),
            ),
            (
                false,
                AnkiLiveResult::NoteExisting(vec![i64::MAX, 9007199254740993]),
                Phase::Existing(vec![i64::MAX, 9007199254740993]),
            ),
            (
                false,
                AnkiLiveResult::NoteUncertain(None),
                Phase::NoteUncertain(None),
            ),
            (
                false,
                AnkiLiveResult::NoteUncertain(Some(i64::MAX)),
                Phase::NoteUncertain(Some(i64::MAX)),
            ),
            (true, AnkiLiveResult::NoSubmission, Phase::NotSubmitted),
            (
                true,
                AnkiLiveResult::ModelCreated(i64::MAX),
                Phase::ModelCreated(i64::MAX),
            ),
            (
                true,
                AnkiLiveResult::ModelUncertain(None),
                Phase::ModelUncertain(None),
            ),
            (
                true,
                AnkiLiveResult::ModelUncertain(Some(i64::MAX)),
                Phase::ModelUncertain(Some(i64::MAX)),
            ),
            (
                false,
                AnkiLiveResult::ModelCreated(i64::MAX),
                Phase::NoteUncertain(None),
            ),
            (
                true,
                AnkiLiveResult::NoteAdded(i64::MAX),
                Phase::ModelUncertain(None),
            ),
        ] {
            let mut model = prepared(effective, create);
            let before = reader_view(&model);
            let (request, _begin) = begin(&mut model);
            assert_eq!(view(&model).phase, Phase::Beginning);
            assert_render(&effects(
                &mut model,
                Event::AnkiLiveBeginFinished(
                    Box::new(request.clone()),
                    StorageResult::AnkiLiveBegun(snapshot(&request)),
                ),
            ));
            assert_eq!(view(&model).phase, Phase::Ready);
            let callback = outcome_to_save(&mut model, result);
            let expected_pair = callback.expected_result(&snapshot(&request)).unwrap();
            let event = Event::AnkiLiveOutcomeFinished(
                Box::new(request),
                StorageResult::AnkiLiveFinished(expected_pair.clone()),
            );
            assert_render(&effects(&mut model, event.clone()));
            assert_eq!(view(&model).phase, expected);
            assert_eq!(reader_view(&model), before);
            assert!(effects(&mut model, event).is_empty());
            assert!(model.take_anki_live_invocation().is_none());
            // Empty recovery later proves resolution, not a new live success.
            model.anki_recovery = AnkiRecoveryState::Restored(Box::default());
            assert_eq!(view(&model).phase, expected);
        }
    }
}

#[test]
fn anki_export_view_storage_failures_hide_provider_success_and_retry_cannot_replay() {
    for effective in [false, true] {
        for fail_begin in [false, true] {
            let mut model = prepared(effective, false);
            let request = if fail_begin {
                begin(&mut model).0
            } else {
                ready(&mut model)
            };
            let late = if fail_begin {
                Event::AnkiLiveBeginFinished(
                    Box::new(request.clone()),
                    StorageResult::AnkiLiveBegun(snapshot(&request)),
                )
            } else {
                outcome_to_save(&mut model, AnkiLiveResult::NoteAdded(i64::MAX));
                Event::AnkiLiveOutcomeFinished(
                    Box::new(request.clone()),
                    StorageResult::AnkiLiveFinished(Box::default()),
                )
            };
            let failure = if fail_begin {
                Event::AnkiLiveBeginFinished(
                    Box::new(request),
                    StorageResult::Failed(DocumentError::Unavailable),
                )
            } else {
                Event::AnkiLiveOutcomeFinished(
                    Box::new(request),
                    StorageResult::Failed(DocumentError::Unavailable),
                )
            };
            assert_render(&effects(&mut model, failure));
            assert_eq!(
                view(&model).phase,
                Phase::StorageFailed(DocumentError::Unavailable)
            );
            assert!(effects(&mut model, late.clone()).is_empty());
            let output = effects(&mut model, Event::RetryAnkiRecovery);
            assert!(output.iter().any(|effect| matches!(effect, Effect::Storage(request) if request.operation == StorageOperation::LoadAnkiRecovery)));
            assert!(Yomoku.view(&model).anki_export.is_none());
            assert!(effects(&mut model, late).is_empty());
            assert!(effects(&mut model, Event::DispatchAnkiLive).is_empty());
        }
    }
}

#[test]
fn anki_export_view_foreign_changed_and_wrong_outcome_acknowledgements_cannot_show_success() {
    for effective in [false, true] {
        let mut model = prepared(effective, false);
        let request = ready(&mut model);
        outcome_to_save(&mut model, AnkiLiveResult::NoteAdded(i64::MAX));
        let foreign = ready(&mut prepared(effective, false));
        assert!(effects(
            &mut model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(foreign),
                StorageResult::AnkiLiveFinished(Box::default())
            )
        )
        .is_empty());
        assert_eq!(view(&model).phase, Phase::Saving);
        assert_render(&effects(
            &mut model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(request.clone()),
                StorageResult::AnkiLiveFinished(snapshot(&request)),
            ),
        ));
        assert_eq!(
            view(&model).phase,
            Phase::StorageFailed(DocumentError::Unavailable)
        );
    }
}

#[test]
fn anki_export_view_restoring_pending_journals_never_recreates_a_live_presentation() {
    let mut previous = prepared(true, false);
    let request = ready(&mut previous);
    let mut model = Model::default();
    effects(&mut model, Event::RestoreAnkiRecovery);
    effects(
        &mut model,
        Event::AnkiRecoveryFinished(1, StorageResult::AnkiRecoveryLoaded(snapshot(&request))),
    );
    assert!(Yomoku.view(&model).anki_export.is_none());
    assert!(effects(&mut model, Event::DispatchAnkiLive).is_empty());
    assert!(effects(&mut model, Event::BeginAnkiLive).is_empty());
}
