use super::*;
use crate::{
    anki::{
        AnkiModelCreation, AnkiModelLayout, AnkiModelRecovery, AnkiPreflightDeck,
        AnkiPreflightModel, AnkiPreflightObservation, AnkiReconciliation, AnkiRecoveryCommit,
        AnkiRecoveryObservation, AnkiRecoveryReadRequest, AnkiRecoveryReadTarget,
        AnkiRecoverySnapshot,
    },
    app::anki::test_support::fixture,
};

fn update(model: &mut Model, event: Event) {
    assert!(Yomoku.update(event, model).effects().next().is_none());
}

fn prepared(effective: bool) -> Model {
    let (mut model, selection) = fixture(effective);
    update(
        &mut model,
        Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        },
    );
    model
}

fn begin(model: &mut Model) -> AnkiPreflightRequest {
    update(model, Event::BeginAnkiExportPreflight);
    model.pending_anki_export_preflight().unwrap().clone()
}

fn finish(
    model: &mut Model,
    request: &AnkiPreflightRequest,
    result: Result<AnkiPreflightObservation, AnkiExportError>,
) {
    update(
        model,
        Event::AnkiExportPreflightFinished(Box::new(request.clone()), result),
    );
}

fn observation() -> AnkiPreflightObservation {
    AnkiPreflightObservation {
        decks: vec![AnkiPreflightDeck {
            id: i64::MAX,
            filtered: false,
        }],
        duplicate_note_ids: vec![],
        models: vec![],
    }
}

fn candidate(id: i64, matches_layout: bool) -> AnkiPreflightModel {
    AnkiPreflightModel {
        id,
        name: AnkiModelLayout::yomoku().name,
        matches_layout,
    }
}

fn recovery() -> Box<AnkiRecoverySnapshot> {
    Box::new(AnkiRecoverySnapshot {
        note: None,
        model: Some(AnkiModelRecovery {
            id: 1.try_into().unwrap(),
            creation: AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
            known_model_id: None,
        }),
    })
}

fn close_gate(model: &mut Model, kind: usize) {
    match kind {
        0 => model.anki_recovery = AnkiRecoveryState::Unrestored,
        1 => model.anki_recovery = AnkiRecoveryState::Loading,
        2 => model.anki_recovery = AnkiRecoveryState::Failed(DocumentError::Unavailable),
        3 => model.anki_recovery = AnkiRecoveryState::Restored(recovery()),
        4 => {
            model.anki_recovery_commit = Some((
                1,
                Box::new(AnkiRecoveryCommit {
                    expected: recovery(),
                    observation: AnkiRecoveryObservation::Note(AnkiReconciliation::Unresolved),
                }),
            ));
        }
        5 => model.anki_recovery_commit_error = Some(DocumentError::Unavailable),
        6 => {
            model.anki_recovery_read =
                AnkiRecoveryReadRequest::new(recovery(), AnkiRecoveryReadTarget::Model);
        }
        7 => model.anki_recovery_read_error = Some(AnkiExportError::PermissionDenied),
        _ => unreachable!(),
    }
}

#[test]
fn anki_preflight_freezes_both_selection_kinds_and_survives_reader_departure() {
    for effective in [false, true] {
        let mut model = prepared(effective);
        let before = Yomoku.view(&model);
        let request = begin(&mut model);
        assert_eq!(request.export(), model.prepared_anki_export().unwrap());
        assert_eq!(request.export().note().surface().as_str(), "猫");
        assert_eq!(request.export().note().fields()[1], "ねこ");
        assert_eq!(
            request.export().note().fields()[3],
            "猫と𠮷と<strong class=\"yomoku-target\">猫</strong>。"
        );
        assert_eq!(request.export().layout(), &AnkiModelLayout::yomoku());
        assert_eq!(i64::from(request.export().deck()), i64::MAX);
        assert_eq!(Yomoku.view(&model), before);
        update(&mut model, Event::BeginAnkiExportPreflight);
        assert_eq!(model.pending_anki_export_preflight(), Some(&request));
        model.clear_reader_word();
        model.close_document();
        // This read owns no reader slot; an unrelated reader failure is retained.
        model.storage_busy = true;
        model.document_error = Some(DocumentError::MissingAnalysis);
        let before = Yomoku.view(&model);
        finish(&mut model, &request, Ok(observation()));
        assert_eq!(Yomoku.view(&model), before);
        assert_eq!(
            model.anki_export_preflight_result(),
            Some(&Ok(AnkiPreflightPlan::CreateModel))
        );
        assert!(model.pending_anki_export_preflight().is_none());
        assert!(model.anki_recovery.allows_preparation());
    }
}

#[test]
fn anki_preflight_begin_and_retry_require_frozen_intent_and_empty_restored_journals() {
    let mut model = Model::default();
    update(&mut model, Event::BeginAnkiExportPreflight);
    update(&mut model, Event::RetryAnkiExportPreflight);
    assert!(model.pending_anki_export_preflight().is_none());
    for kind in 0..8 {
        let mut model = prepared(false);
        close_gate(&mut model, kind);
        update(&mut model, Event::BeginAnkiExportPreflight);
        assert!(model.pending_anki_export_preflight().is_none());
        let mut model = prepared(false);
        let request = begin(&mut model);
        close_gate(&mut model, kind);
        update(&mut model, Event::RetryAnkiExportPreflight);
        assert_eq!(model.pending_anki_export_preflight(), Some(&request));
    }
}

#[test]
fn anki_preflight_changed_journal_gates_or_intent_reject_even_matching_callbacks() {
    for kind in 0..9 {
        let mut model = prepared(false);
        let request = begin(&mut model);
        if kind < 8 {
            close_gate(&mut model, kind);
        } else {
            model.prepared_anki_export = None;
        }
        let journals = model.anki_recovery.clone();
        let before = Yomoku.view(&model);
        finish(&mut model, &request, Ok(observation()));
        assert_eq!(
            model.anki_export_preflight_result(),
            Some(&Err(AnkiExportError::InvalidResponse))
        );
        assert_eq!(model.anki_recovery, journals);
        assert_eq!(Yomoku.view(&model), before);
    }
}

#[test]
fn anki_preflight_foreign_duplicate_and_late_callbacks_cannot_consume_current_read() {
    let mut model = prepared(false);
    let request = begin(&mut model);
    let mut foreign = prepared(false);
    let foreign_request = begin(&mut foreign);
    assert_eq!(request.export(), foreign_request.export());
    assert_ne!(request, foreign_request);
    finish(&mut model, &foreign_request, Ok(observation()));
    assert_eq!(model.pending_anki_export_preflight(), Some(&request));
    assert!(model.anki_export_preflight_result().is_none());
    finish(&mut model, &request, Ok(observation()));
    let saved = model.anki_export_preflight_result().cloned();
    finish(&mut model, &request, Err(AnkiExportError::ProviderFailure));
    update(&mut model, Event::BeginAnkiExportPreflight);
    assert_eq!(model.anki_export_preflight_result(), saved.as_ref());
    assert!(model.pending_anki_export_preflight().is_none());
    update(&mut model, Event::RetryAnkiExportPreflight);
    let next = model.pending_anki_export_preflight().unwrap().clone();
    assert_ne!(next, request);
    finish(&mut model, &request, Ok(observation()));
    assert_eq!(model.pending_anki_export_preflight(), Some(&next));
    finish(&mut model, &next, Err(AnkiExportError::PermissionDenied));
    assert_eq!(
        model.anki_export_preflight_result(),
        Some(&Err(AnkiExportError::PermissionDenied))
    );
}

#[test]
fn anki_preflight_explicit_retry_invalidates_lost_reads_and_failed_observations() {
    let mut model = prepared(false);
    update(&mut model, Event::RetryAnkiExportPreflight);
    assert!(model.pending_anki_export_preflight().is_none());
    let first = begin(&mut model);
    update(&mut model, Event::RetryAnkiExportPreflight);
    let second = model.pending_anki_export_preflight().unwrap().clone();
    assert_ne!(first, second);
    assert_eq!(first.export(), second.export());
    finish(&mut model, &first, Ok(observation()));
    assert_eq!(model.pending_anki_export_preflight(), Some(&second));
    finish(
        &mut model,
        &second,
        Err(AnkiExportError::DuplicateLookupFailed),
    );
    update(&mut model, Event::RetryAnkiExportPreflight);
    let third = model.pending_anki_export_preflight().unwrap().clone();
    assert_ne!(second, third);
    finish(&mut model, &second, Ok(observation()));
    assert_eq!(model.pending_anki_export_preflight(), Some(&third));
    finish(&mut model, &third, Ok(observation()));
    assert_eq!(
        model.anki_export_preflight_result(),
        Some(&Ok(AnkiPreflightPlan::CreateModel))
    );
}

#[test]
fn anki_preflight_clear_invalidates_read_but_preserves_durable_obligations() {
    let mut model = prepared(false);
    let old = begin(&mut model);
    close_gate(&mut model, 3);
    close_gate(&mut model, 4);
    let recovery = model.anki_recovery.clone();
    let commit = model.anki_recovery_commit.clone();
    update(&mut model, Event::ClearPreparedAnkiExport);
    finish(&mut model, &old, Ok(observation()));
    assert!(model.pending_anki_export_preflight().is_none());
    assert!(model.anki_export_preflight_result().is_none());
    assert!(model.prepared_anki_export().is_none());
    assert_eq!(model.anki_recovery, recovery);
    assert_eq!(model.anki_recovery_commit, commit);
    model.anki_recovery = AnkiRecoveryState::Restored(Box::default());
    model.anki_recovery_commit = None;
    update(
        &mut model,
        Event::PrepareAnkiExport {
            selection: old.export().selection(),
            deck: old.export().deck(),
        },
    );
    let next = begin(&mut model);
    assert_eq!(old.export(), next.export());
    assert_ne!(old, next);
    finish(&mut model, &old, Ok(observation()));
    assert_eq!(model.pending_anki_export_preflight(), Some(&next));
    let restarted = Model::default();
    assert!(restarted.prepared_anki_export().is_none());
    assert!(restarted.pending_anki_export_preflight().is_none());
    assert!(restarted.anki_export_preflight_result().is_none());
}

#[test]
fn anki_preflight_model_discovery_requires_one_exact_name_and_full_layout_match() {
    let unrelated = AnkiPreflightModel {
        id: 1,
        name: "User model".into(),
        matches_layout: false,
    };
    for (models, expected) in [
        (vec![], Ok(AnkiPreflightPlan::CreateModel)),
        (vec![unrelated.clone()], Ok(AnkiPreflightPlan::CreateModel)),
        (
            vec![unrelated, candidate(9007199254740993, true)],
            Ok(AnkiPreflightPlan::ReuseModel(
                9007199254740993.try_into().unwrap(),
            )),
        ),
        (
            vec![candidate(1, false)],
            Err(AnkiExportError::IncompatibleModel),
        ),
        (
            vec![candidate(1, true), candidate(2, true)],
            Err(AnkiExportError::IncompatibleModel),
        ),
        (
            vec![candidate(1, true), candidate(2, false)],
            Err(AnkiExportError::IncompatibleModel),
        ),
    ] {
        let mut model = prepared(false);
        let request = begin(&mut model);
        let mut value = observation();
        value.models = models;
        finish(&mut model, &request, Ok(value));
        assert_eq!(model.anki_export_preflight_result(), Some(&expected));
        assert!(model.anki_recovery.allows_preparation());
    }
}

#[test]
fn anki_preflight_exact_duplicates_win_over_model_conflicts_and_never_grant_insertion() {
    let mut model = prepared(false);
    let request = begin(&mut model);
    let mut value = observation();
    value.models = vec![candidate(1, false), candidate(2, true)];
    value.duplicate_note_ids = vec![i64::MAX, 9007199254740993, i64::MAX];
    let before = Yomoku.view(&model);
    finish(&mut model, &request, Ok(value));
    let Some(Ok(AnkiPreflightPlan::Existing(notes))) = model.anki_export_preflight_result() else {
        panic!("exact existing notes")
    };
    assert_eq!(
        notes
            .ids()
            .iter()
            .copied()
            .map(i64::from)
            .collect::<Vec<_>>(),
        vec![i64::MAX, 9007199254740993]
    );
    assert_eq!(Yomoku.view(&model), before);
    assert!(model.anki_recovery.allows_preparation());
}

#[test]
fn anki_preflight_requires_the_frozen_nonfiltered_deck_without_fallback() {
    for decks in [
        vec![],
        vec![AnkiPreflightDeck {
            id: 1,
            filtered: false,
        }],
        vec![AnkiPreflightDeck {
            id: i64::MAX,
            filtered: true,
        }],
    ] {
        let mut model = prepared(false);
        let request = begin(&mut model);
        let mut value = observation();
        value.decks = decks;
        value.duplicate_note_ids = vec![1];
        finish(&mut model, &request, Ok(value));
        assert_eq!(
            model.anki_export_preflight_result(),
            Some(&Err(AnkiExportError::DeckUnavailable))
        );
    }
}

#[test]
fn anki_preflight_rejects_malformed_complete_observations() {
    let mutations: &[fn(&mut AnkiPreflightObservation)] = &[
        |v| v.decks[0].id = 0,
        |v| v.decks[0].id = -1,
        |v| v.decks.push(v.decks[0].clone()),
        |v| v.duplicate_note_ids = vec![1, 0],
        |v| v.duplicate_note_ids = vec![-1],
        |v| v.models = vec![candidate(0, true)],
        |v| v.models = vec![candidate(-1, true)],
        |v| v.models = vec![candidate(1, true), candidate(1, false)],
        |v| {
            v.models = vec![AnkiPreflightModel {
                id: 1,
                name: String::new(),
                matches_layout: false,
            }]
        },
        |v| {
            v.models = vec![AnkiPreflightModel {
                id: 1,
                name: "Wrong name".into(),
                matches_layout: true,
            }]
        },
    ];
    for mutate in mutations {
        let mut model = prepared(false);
        let request = begin(&mut model);
        let mut value = observation();
        mutate(&mut value);
        finish(&mut model, &request, Ok(value));
        assert_eq!(
            model.anki_export_preflight_result(),
            Some(&Err(AnkiExportError::InvalidResponse))
        );
    }
}

#[test]
fn anki_preflight_provider_errors_never_become_empty_success_or_reader_errors() {
    for error in [
        AnkiExportError::ProviderUnavailable,
        AnkiExportError::UnsupportedProvider,
        AnkiExportError::PermissionDenied,
        AnkiExportError::CollectionUnavailable,
        AnkiExportError::DeckUnavailable,
        AnkiExportError::IncompatibleModel,
        AnkiExportError::DuplicateLookupFailed,
        AnkiExportError::ProviderFailure,
        AnkiExportError::InvalidResponse,
    ] {
        let mut model = prepared(false);
        let request = begin(&mut model);
        let before = Yomoku.view(&model);
        finish(&mut model, &request, Err(error));
        assert_eq!(model.anki_export_preflight_result(), Some(&Err(error)));
        assert!(model.pending_anki_export_preflight().is_none());
        assert_eq!(model.prepared_anki_export(), Some(request.export()));
        assert_eq!(Yomoku.view(&model), before);
        assert!(model.anki_recovery.allows_preparation());
    }
}
