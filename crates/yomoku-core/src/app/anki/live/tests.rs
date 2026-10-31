use super::*;
use crate::{
    anki::{
        AnkiExportError, AnkiLiveCallback, AnkiLiveOutcome, AnkiModelCreationOutcome,
        AnkiModelRecovery, AnkiPreflightDeck, AnkiPreflightModel, AnkiPreflightObservation,
        AnkiRecovery, AnkiRecoveryReadTarget, AnkiRecoverySnapshot, AnkiSubmissionOutcome,
    },
    app::anki::test_support::fixture,
};

fn update(model: &mut Model, event: Event) {
    let mut command = Yomoku.update(event, model);
    assert!(command.effects().next().is_none());
}

fn prepared(effective: bool, create: bool) -> Model {
    let (mut model, selection) = fixture(effective);
    update(
        &mut model,
        Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        },
    );
    let _command = Yomoku.update(Event::BeginAnkiExportPreflight, &mut model);
    let request = model.pending_anki_export_preflight().unwrap().clone();
    update(
        &mut model,
        Event::AnkiExportPreflightFinished(
            Box::new(request),
            Ok(AnkiPreflightObservation {
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
                        name: crate::anki::MODEL_NAME.into(),
                        matches_layout: true,
                    }]
                },
            }),
        ),
    );
    model
}

fn begin(model: &mut Model) -> (AnkiLiveRequest, Command<Effect, Event>) {
    let mut command = Yomoku.update(Event::BeginAnkiLive, model);
    let Some(Effect::Storage(effect)) = command.effects().next() else {
        panic!("begin storage")
    };
    let StorageOperation::BeginAnkiLive(request) = effect.operation else {
        panic!("begin")
    };
    assert_eq!(
        model.anki_live_progress(),
        Some(AnkiLiveProgress::Beginning)
    );
    (*request, command)
}

fn snapshot(request: &AnkiLiveRequest) -> Box<AnkiRecoverySnapshot> {
    let mut snapshot = AnkiRecoverySnapshot::default();
    match request.intent() {
        AnkiLiveIntent::Note(payload) => {
            snapshot.note = Some(AnkiRecovery {
                id: 1.try_into().unwrap(),
                submission: *payload.clone(),
                known_note_id: None,
            })
        }
        AnkiLiveIntent::Model(payload) => {
            snapshot.model = Some(AnkiModelRecovery {
                id: 1.try_into().unwrap(),
                creation: *payload.clone(),
                known_model_id: None,
            })
        }
    }
    Box::new(snapshot)
}

fn ready(model: &mut Model) -> AnkiLiveRequest {
    let (request, _command) = begin(model);
    update(
        model,
        Event::AnkiLiveBeginFinished(
            Box::new(request.clone()),
            StorageResult::AnkiLiveBegun(snapshot(&request)),
        ),
    );
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
    request
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

fn save(model: &mut Model, callback: AnkiLiveCallback) -> Command<Effect, Event> {
    let mut command = Yomoku.update(Event::FinishAnkiLive(Box::new(callback)), model);
    assert!(
        matches!(command.effects().next(), Some(Effect::Storage(effect)) if matches!(effect.operation, StorageOperation::FinishAnkiLive(_)))
    );
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Saving));
    command
}

#[test]
fn anki_live_frozen_both_selections_and_kinds_issue_one_permit_without_reader_writes() {
    for effective in [false, true] {
        for create in [false, true] {
            let mut model = prepared(effective, create);
            let reader_before = Yomoku.view(&model);
            let frozen = model.prepared_anki_export().unwrap().clone();
            let request = ready(&mut model);
            assert_eq!(Yomoku.view(&model), reader_before);
            assert!(!model.anki_recovery().allows_preparation());
            let _navigation = Yomoku.update(Event::OpenSettings, &mut model);
            let invocation = model.take_anki_live_invocation().unwrap();
            match invocation.intent() {
                AnkiLiveIntent::Note(note) => {
                    assert_eq!(note.note(), frozen.note());
                    assert_eq!(note.deck(), frozen.deck());
                    assert_eq!(note.layout(), frozen.layout());
                    assert_eq!(i64::from(note.model()), 9007199254740993);
                }
                AnkiLiveIntent::Model(creation) => assert_eq!(creation.layout(), frozen.layout()),
            }
            assert!(model.take_anki_live_invocation().is_none());
            let before = Yomoku.view(&model);
            let callback = invocation.complete(terminal(create));
            let _command = save(&mut model, callback.clone());
            assert_eq!(Yomoku.view(&model), before);
            update(
                &mut model,
                Event::FinishAnkiLive(Box::new(callback.clone())),
            );
            assert!(!model.anki_recovery().allows_preparation());
            update(
                &mut model,
                Event::AnkiLiveOutcomeFinished(
                    Box::new(request),
                    StorageResult::AnkiLiveFinished(
                        callback
                            .expected_result(&snapshot(callback.request()))
                            .unwrap(),
                    ),
                ),
            );
            assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Finished));
            assert!(model.anki_recovery().allows_preparation());
            assert!(model.take_anki_live_invocation().is_none());
            assert_eq!(Yomoku.view(&model), before);
            update(&mut model, Event::BeginAnkiLive);
            update(&mut model, Event::ClearPreparedAnkiExport);
            assert!(model.prepared_anki_export().is_none());
            assert!(model.anki_live_progress().is_none());
        }
    }
}

#[test]
fn anki_live_begin_requires_successful_current_preflight_and_all_journal_gates() {
    update(&mut Model::default(), Event::BeginAnkiLive);
    for closed in 0..10 {
        let mut model = prepared(false, false);
        match closed {
            0 => model.anki_recovery = AnkiRecoveryState::Unrestored,
            1 => model.anki_recovery = AnkiRecoveryState::Failed(DocumentError::Unavailable),
            2 => model.anki_recovery_commit_error = Some(DocumentError::Unavailable),
            3 => model.anki_recovery_read_error = Some(AnkiExportError::ProviderFailure),
            4 => model.prepared_anki_export = None,
            5 => model.anki_export_preflight = None,
            6 => model.anki_export_preflight.as_mut().unwrap().result = None,
            7 => {
                model.anki_export_preflight.as_mut().unwrap().result =
                    Some(Err(AnkiExportError::PermissionDenied))
            }
            8 => {
                model.anki_export_preflight.as_mut().unwrap().result =
                    Some(Ok(AnkiPreflightPlan::Existing(
                        crate::anki::AnkiExistingNotes::new(vec![1.try_into().unwrap()]).unwrap(),
                    )))
            }
            9 => {
                let request = AnkiLiveRequest::new(AnkiLiveIntent::Model(Box::new(
                    AnkiModelCreation::new(crate::anki::AnkiModelLayout::yomoku()).unwrap(),
                )));
                model.anki_recovery = AnkiRecoveryState::Restored(snapshot(&request));
            }
            _ => unreachable!(),
        }
        update(&mut model, Event::BeginAnkiLive);
        assert!(model.anki_live_progress().is_none());
        assert!(model.take_anki_live_invocation().is_none());
    }
}

#[test]
fn anki_live_foreign_duplicate_and_old_begin_callbacks_cannot_grant_a_permit() {
    let mut model = prepared(false, false);
    let (request, _command) = begin(&mut model);
    let mut foreign = prepared(false, false);
    let (other, _other_command) = begin(&mut foreign);
    assert_eq!(request.intent(), other.intent());
    assert_ne!(request, other);
    update(
        &mut model,
        Event::AnkiLiveBeginFinished(
            Box::new(other),
            StorageResult::AnkiLiveBegun(snapshot(&request)),
        ),
    );
    assert_eq!(
        model.anki_live_progress(),
        Some(AnkiLiveProgress::Beginning)
    );
    assert!(model.take_anki_live_invocation().is_none());
    update(
        &mut model,
        Event::AnkiLiveBeginFinished(
            Box::new(request.clone()),
            StorageResult::AnkiLiveBegun(snapshot(&request)),
        ),
    );
    let invocation = model.take_anki_live_invocation().unwrap();
    update(
        &mut model,
        Event::AnkiLiveBeginFinished(
            Box::new(request.clone()),
            StorageResult::AnkiLiveBegun(snapshot(&request)),
        ),
    );
    assert!(model.take_anki_live_invocation().is_none());
    let _command = save(&mut model, invocation.complete(terminal(false)));
}

#[test]
fn anki_live_invalid_begin_success_and_changed_gates_require_read_only_restore() {
    for invalid in 0..6 {
        let mut model = prepared(false, false);
        let (request, _command) = begin(&mut model);
        let mut pair = snapshot(&request);
        let result = match invalid {
            0 => StorageResult::AnkiLiveBegun(Box::default()),
            1 => {
                pair.note.as_mut().unwrap().known_note_id = Some(1.try_into().unwrap());
                StorageResult::AnkiLiveBegun(pair)
            }
            2 => StorageResult::AnkiLiveFinished(pair),
            3 => StorageResult::Failed(DocumentError::IncompatibleDatabase),
            4 => {
                model.anki_recovery_commit_error = Some(DocumentError::Unavailable);
                StorageResult::AnkiLiveBegun(pair)
            }
            5 => {
                model.anki_recovery = AnkiRecoveryState::Loading;
                StorageResult::AnkiLiveBegun(pair)
            }
            _ => unreachable!(),
        };
        update(
            &mut model,
            Event::AnkiLiveBeginFinished(Box::new(request), result),
        );
        assert!(matches!(
            model.anki_live_progress(),
            Some(AnkiLiveProgress::Failed(_))
        ));
        assert!(model.take_anki_live_invocation().is_none());
        update(&mut model, Event::BeginAnkiLive);
        update(&mut model, Event::ClearPreparedAnkiExport);
        assert!(model.prepared_anki_export().is_some());
    }
}

#[test]
fn anki_live_active_stages_block_clear_recovery_and_preflight_without_blocking_reader() {
    for stage in 0..4 {
        let mut model = prepared(true, false);
        if stage == 0 {
            let _ = begin(&mut model);
        } else {
            ready(&mut model);
            if stage >= 2 {
                let invocation = model.take_anki_live_invocation().unwrap();
                if stage == 3 {
                    let _ = save(&mut model, invocation.complete(terminal(false)));
                }
            }
        }
        let progress = model.anki_live_progress();
        let frozen = model.prepared_anki_export().cloned();
        for event in [
            Event::ClearPreparedAnkiExport,
            Event::BeginAnkiLive,
            Event::BeginAnkiExportPreflight,
            Event::RetryAnkiExportPreflight,
            Event::RetryAnkiRecovery,
            Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Note),
        ] {
            update(&mut model, event);
            assert_eq!(model.anki_live_progress(), progress);
            assert_eq!(model.prepared_anki_export(), frozen.as_ref());
        }
        let _command = Yomoku.update(Event::OpenSettings, &mut model);
        assert!(Yomoku.view(&model).settings_open);
        assert!(!Yomoku.view(&model).storage_busy);
    }
}

#[test]
fn anki_live_every_outcome_is_accepted_only_after_exact_committed_pair() {
    let outcomes = [
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::NotInserted),
        terminal(false),
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Existing(
            crate::anki::AnkiExistingNotes::new(vec![
                1.try_into().unwrap(),
                i64::MAX.try_into().unwrap(),
            ])
            .unwrap(),
        )),
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Uncertain {
            known_note_id: None,
        }),
        AnkiLiveOutcome::Note(AnkiSubmissionOutcome::Uncertain {
            known_note_id: Some(i64::MAX.try_into().unwrap()),
        }),
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::NotCreated),
        terminal(true),
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Uncertain {
            known_model_id: None,
        }),
        AnkiLiveOutcome::Model(AnkiModelCreationOutcome::Uncertain {
            known_model_id: Some(i64::MAX.try_into().unwrap()),
        }),
    ];
    for outcome in outcomes {
        let create = matches!(outcome, AnkiLiveOutcome::Model(_));
        let mut model = prepared(false, create);
        let request = ready(&mut model);
        let before = model.anki_recovery().clone();
        let invocation = model.take_anki_live_invocation().unwrap();
        let callback = invocation.complete(outcome);
        let expected = callback.expected_result(&snapshot(&request)).unwrap();
        let _command = save(&mut model, callback);
        assert_eq!(model.anki_recovery(), &before);
        update(
            &mut model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(request.clone()),
                StorageResult::AnkiLiveFinished(expected.clone()),
            ),
        );
        assert_eq!(
            model.anki_recovery(),
            &AnkiRecoveryState::Restored(expected)
        );
        update(
            &mut model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(request),
                StorageResult::Failed(DocumentError::Unavailable),
            ),
        );
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Finished));
        assert!(model.take_anki_live_invocation().is_none());
    }
}

#[test]
fn anki_live_wrong_kind_foreign_and_duplicate_outcomes_cannot_erase_obligations() {
    for create in [false, true] {
        let mut model = prepared(false, create);
        ready(&mut model);
        let mut other = prepared(false, create);
        ready(&mut other);
        let foreign = other
            .take_anki_live_invocation()
            .unwrap()
            .complete(terminal(create));
        update(&mut model, Event::FinishAnkiLive(Box::new(foreign)));
        assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
        let before = model.anki_recovery().clone();
        let wrong = model
            .take_anki_live_invocation()
            .unwrap()
            .complete(terminal(!create));
        update(&mut model, Event::FinishAnkiLive(Box::new(wrong)));
        assert_eq!(model.anki_recovery(), &before);
        assert_eq!(
            model.anki_live_progress(),
            Some(AnkiLiveProgress::Failed(DocumentError::Unavailable))
        );
        assert!(model.take_anki_live_invocation().is_none());
    }
}

#[test]
fn anki_live_lost_begin_invocation_and_outcome_never_restore_mutation_authority() {
    for stage in 0..4 {
        let mut model = prepared(false, false);
        let (request, _command) = begin(&mut model);
        let begun = snapshot(&request);
        let mut callback = None;
        if stage != 0 {
            update(
                &mut model,
                Event::AnkiLiveBeginFinished(
                    Box::new(request.clone()),
                    StorageResult::AnkiLiveBegun(begun.clone()),
                ),
            );
            if stage >= 2 {
                callback = Some(
                    model
                        .take_anki_live_invocation()
                        .unwrap()
                        .complete(terminal(false)),
                );
                if stage == 3 {
                    let _ = save(&mut model, callback.clone().unwrap());
                }
            }
        }
        update(&mut model, Event::AbandonAnkiLive);
        let mut retry = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
        assert!(
            matches!(retry.effects().next(), Some(Effect::Storage(effect)) if effect.operation == StorageOperation::LoadAnkiRecovery)
        );
        assert!(model.prepared_anki_export().is_none());
        assert!(model.anki_export_preflight_result().is_none());
        update(
            &mut model,
            Event::AnkiLiveBeginFinished(
                Box::new(request.clone()),
                StorageResult::AnkiLiveBegun(begun.clone()),
            ),
        );
        if let Some(callback) = callback {
            update(&mut model, Event::FinishAnkiLive(Box::new(callback)));
        }
        update(
            &mut model,
            Event::AnkiLiveOutcomeFinished(
                Box::new(request),
                StorageResult::AnkiLiveFinished(Box::default()),
            ),
        );
        assert!(model.take_anki_live_invocation().is_none());
        let id = model.anki_recovery_request.unwrap();
        update(
            &mut model,
            Event::AnkiRecoveryFinished(id, StorageResult::AnkiRecoveryLoaded(begun)),
        );
        assert!(!model.anki_recovery().allows_preparation());
        update(&mut model, Event::BeginAnkiLive);
        let mut restarted = Model::default();
        assert!(restarted.take_anki_live_invocation().is_none());
    }
}

#[test]
fn anki_live_failed_or_invalid_outcome_commit_retains_pair_until_explicit_restore() {
    for invalid in 0..4 {
        let mut model = prepared(false, false);
        let request = ready(&mut model);
        let callback = model
            .take_anki_live_invocation()
            .unwrap()
            .complete(terminal(false));
        let _command = save(&mut model, callback);
        let before = model.anki_recovery().clone();
        let result = match invalid {
            0 => StorageResult::Failed(DocumentError::Unavailable),
            1 => StorageResult::AnkiLiveFinished(snapshot(&request)),
            2 => StorageResult::AnkiLiveBegun(Box::default()),
            3 => StorageResult::ImportCancelled,
            _ => unreachable!(),
        };
        update(
            &mut model,
            Event::AnkiLiveOutcomeFinished(Box::new(request), result),
        );
        assert_eq!(model.anki_recovery(), &before);
        assert!(matches!(
            model.anki_live_progress(),
            Some(AnkiLiveProgress::Failed(_))
        ));
        assert!(model.take_anki_live_invocation().is_none());
        let _retry = Yomoku.update(Event::RetryAnkiRecovery, &mut model);
        let id = model.anki_recovery_request.unwrap();
        update(
            &mut model,
            Event::AnkiRecoveryFinished(id, StorageResult::AnkiRecoveryLoaded(Box::default())),
        );
        assert!(model.anki_recovery().allows_preparation());
        update(&mut model, Event::BeginAnkiLive); // no restored preflight or intent
    }
}

#[test]
fn anki_live_native_operations_results_and_callbacks_have_no_shell_codec() {
    use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
    let mut model = prepared(false, false);
    let request = ready(&mut model);
    let callback = model
        .take_anki_live_invocation()
        .unwrap()
        .complete(terminal(false));
    for event in [
        Event::BeginAnkiLive,
        Event::AbandonAnkiLive,
        Event::FinishAnkiLive(Box::new(callback.clone())),
        Event::AnkiLiveBeginFinished(
            Box::new(request.clone()),
            StorageResult::AnkiLiveBegun(snapshot(&request)),
        ),
        Event::AnkiLiveOutcomeFinished(
            Box::new(request.clone()),
            StorageResult::AnkiLiveFinished(Box::default()),
        ),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
    for operation in [
        StorageOperation::BeginAnkiLive(Box::new(request)),
        StorageOperation::FinishAnkiLive(Box::new(callback)),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    }
    for result in [
        StorageResult::AnkiLiveBegun(Box::default()),
        StorageResult::AnkiLiveFinished(Box::default()),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
    }
}

#[test]
fn anki_live_crux_callbacks_keep_the_captured_begin_and_outcome_owner() {
    let mut model = prepared(false, false);
    let mut command = Yomoku.update(Event::BeginAnkiLive, &mut model);
    let Some(Effect::Storage(mut effect)) = command.effects().next() else {
        panic!("begin")
    };
    let StorageOperation::BeginAnkiLive(request) = &effect.operation else {
        panic!("begin")
    };
    let pair = snapshot(request);
    effect.resolve(StorageResult::AnkiLiveBegun(pair)).unwrap();
    update(&mut model, command.events().next().unwrap());
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
    let callback = model
        .take_anki_live_invocation()
        .unwrap()
        .complete(terminal(false));
    let mut command = Yomoku.update(Event::FinishAnkiLive(Box::new(callback)), &mut model);
    let Some(Effect::Storage(mut effect)) = command.effects().next() else {
        panic!("finish")
    };
    effect
        .resolve(StorageResult::AnkiLiveFinished(Box::default()))
        .unwrap();
    update(&mut model, command.events().next().unwrap());
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Finished));
    assert!(model.anki_recovery().allows_preparation());
    assert!(model.take_anki_live_invocation().is_none());
}

#[test]
fn anki_live_take_and_outcome_ack_recheck_pair_and_reject_foreign_owners() {
    let mut model = prepared(false, false);
    let request = ready(&mut model);
    let pair = model.anki_recovery.clone();
    model.anki_recovery = AnkiRecoveryState::Restored(Box::default());
    assert!(model.take_anki_live_invocation().is_none());
    model.anki_recovery = pair.clone();
    let callback = model
        .take_anki_live_invocation()
        .unwrap()
        .complete(terminal(false));
    let _command = save(&mut model, callback);
    let mut foreign = prepared(false, false);
    let (foreign, _command) = begin(&mut foreign);
    update(
        &mut model,
        Event::AnkiLiveOutcomeFinished(
            Box::new(foreign),
            StorageResult::AnkiLiveFinished(Box::default()),
        ),
    );
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Saving));
    assert_eq!(model.anki_recovery(), &pair);
    model.anki_recovery = AnkiRecoveryState::Failed(DocumentError::Unavailable);
    update(
        &mut model,
        Event::AnkiLiveOutcomeFinished(
            Box::new(request),
            StorageResult::AnkiLiveFinished(Box::default()),
        ),
    );
    assert!(matches!(
        model.anki_live_progress(),
        Some(AnkiLiveProgress::Failed(_))
    ));
    assert!(!model.anki_recovery().allows_preparation());
}
