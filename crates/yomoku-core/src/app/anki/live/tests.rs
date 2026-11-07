use super::*;
use crate::anki::*;
use crate::app::anki::test_support::fixture;

fn update(model: &mut Model, event: Event) {
    drop(Yomoku.update(event, model));
}
fn observation(ids: Vec<i64>) -> AnkiPreflightResult {
    AnkiPreflightResult::Complete(AnkiPreflightObservation {
        decks: vec![AnkiPreflightDeck {
            id: 7,
            filtered: false,
        }],
        duplicate_note_ids: ids,
        models: vec![AnkiPreflightModel {
            id: 9,
            name: MODEL_NAME.into(),
            matches_layout: true,
        }],
    })
}
fn prepare(model: &mut Model, selection: AnkiSelection) -> AnkiPreflightRequest {
    update(
        model,
        Event::PrepareAnkiExport {
            selection,
            deck: 7.try_into().unwrap(),
        },
    );
    update(model, Event::BeginAnkiExportPreflight);
    model.pending_anki_export_preflight().unwrap().clone()
}
fn ready() -> (Model, AnkiSelection) {
    let (mut model, selection) = fixture();
    let read = prepare(&mut model, selection);
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(Box::new(read), observation(vec![])),
    );
    update(&mut model, Event::BeginAnkiLive);
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
    (model, selection)
}
fn dispatch(model: &mut Model) -> Command<Effect, Event> {
    Yomoku.update(Event::DispatchAnkiLive, model)
}
fn callback(command: &mut Command<Effect, Event>, result: AnkiLiveResult) -> Event {
    let mut request = command
        .effects()
        .find_map(|effect| match effect {
            Effect::AnkiLive(r) => Some(r),
            _ => None,
        })
        .unwrap();
    request.resolve(result).unwrap();
    command.events().next().unwrap()
}

#[test]
fn direct_provider_effect_consumes_one_owner_and_ignores_repeated_taps_and_callbacks() {
    let (mut model, _) = ready();
    let frozen = model.prepared_anki_export().unwrap().clone();
    for _ in 0..3 {
        update(&mut model, Event::BeginAnkiLive);
    }
    let mut command = dispatch(&mut model);
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Invoking));
    assert!(dispatch(&mut model).effects().next().is_none());
    let done = callback(&mut command, AnkiLiveResult::NoteAdded(88));
    update(&mut model, Event::DismissReaderWord);
    assert_eq!(model.prepared_anki_export(), Some(&frozen));
    update(&mut model, done.clone());
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::NoteAdded(88)
    );
    update(&mut model, done);
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::NoteAdded(88)
    );
    assert!(dispatch(&mut model).effects().next().is_none());
}

#[test]
fn lost_or_failed_query_never_implies_absence_and_retry_uses_a_new_read_owner() {
    let (mut model, selection) = fixture();
    let old = prepare(&mut model, selection);
    update(&mut model, Event::RetryAnkiExportPreflight);
    let fresh = model.pending_anki_export_preflight().unwrap().clone();
    assert_ne!(old, fresh);
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(Box::new(old), observation(vec![])),
    );
    assert!(model.pending_anki_export_preflight().is_some());
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(
            Box::new(fresh),
            AnkiPreflightResult::Failed(AnkiExportError::DuplicateLookupFailed),
        ),
    );
    update(&mut model, Event::BeginAnkiLive);
    assert!(model.anki_live.is_none());
    assert!(matches!(
        model.anki_export_preflight_result(),
        Some(Err(AnkiExportError::DuplicateLookupFailed))
    ));
}

#[test]
fn exact_existing_notes_created_or_deleted_externally_are_observed_by_fresh_queries() {
    let (mut model, selection) = fixture();
    let read = prepare(&mut model, selection);
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(Box::new(read), observation(vec![88, 99])),
    );
    update(&mut model, Event::BeginAnkiLive);
    assert!(model.anki_live.is_none());
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::Existing(vec![88, 99])
    );
    update(&mut model, Event::RetryAnkiExportPreflight);
    let read = model.pending_anki_export_preflight().unwrap().clone();
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(Box::new(read), observation(vec![])),
    );
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::ReuseModel(9)
    );
    update(&mut model, Event::BeginAnkiLive);
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Ready));
}

#[test]
fn uncertain_write_requires_explicit_clear_fresh_queries_and_new_action_without_replay() {
    let (mut model, selection) = ready();
    let mut command = dispatch(&mut model);
    let done = callback(&mut command, AnkiLiveResult::NoteUncertain(Some(88)));
    update(&mut model, done.clone());
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::NoteUncertain(Some(88))
    );
    assert!(dispatch(&mut model).effects().next().is_none());
    update(&mut model, Event::ClearPreparedAnkiExport);
    let read = prepare(&mut model, selection);
    update(
        &mut model,
        Event::AnkiExportPreflightProviderFinished(Box::new(read), observation(vec![])),
    );
    assert!(model.anki_live.is_none());
    update(&mut model, Event::BeginAnkiLive);
    let mut fresh = dispatch(&mut model);
    update(&mut model, done);
    assert_eq!(model.anki_live_progress(), Some(AnkiLiveProgress::Invoking));
    let done = callback(&mut fresh, AnkiLiveResult::NoteAdded(99));
    update(&mut model, done);
    assert_eq!(
        model.anki_export_view().unwrap().phase,
        AnkiExportPhase::NoteAdded(99)
    );
}

#[test]
fn restart_has_no_recovery_or_automatic_write_and_setup_is_a_fresh_provider_query() {
    let (mut previous, _) = ready();
    let _pending = dispatch(&mut previous);
    let mut restarted = Model::default();
    assert!(dispatch(&mut restarted).effects().next().is_none());
    assert!(restarted.prepared_anki_export().is_none());
    restarted.anki_settings.phase = AnkiSettingsPhase::Ready;
    restarted.anki_settings.default_deck_id = Some(7);
    let mut query = Yomoku.update(Event::CheckAnkiSetup, &mut restarted);
    assert!(query
        .effects()
        .any(|effect| matches!(effect, Effect::AnkiExportPreflight(_))));
    assert!(restarted.anki_live.is_none());
}

#[test]
fn incompatible_ambiguous_and_partial_model_observations_never_authorize_writes() {
    for models in [
        vec![AnkiPreflightModel {
            id: 9,
            name: MODEL_NAME.into(),
            matches_layout: false,
        }],
        vec![
            AnkiPreflightModel {
                id: 9,
                name: MODEL_NAME.into(),
                matches_layout: true,
            },
            AnkiPreflightModel {
                id: 10,
                name: MODEL_NAME.into(),
                matches_layout: true,
            },
        ],
    ] {
        let observation = AnkiPreflightObservation {
            decks: vec![AnkiPreflightDeck {
                id: 7,
                filtered: false,
            }],
            duplicate_note_ids: vec![],
            models,
        };
        assert_eq!(
            evaluate_layout(&AnkiModelLayout::yomoku(), 7, observation),
            Err(AnkiExportError::IncompatibleModel)
        );
    }
}

#[test]
fn confirmed_add_notice_is_consumed_once_and_duplicate_or_uncertain_results_are_silent() {
    let (mut model, _) = ready();
    let mut command = dispatch(&mut model);
    let done = callback(&mut command, AnkiLiveResult::NoteAdded(88));
    update(&mut model, done.clone());
    let notice = Yomoku.view(&model).anki_added_notice.unwrap();
    update(&mut model, Event::DismissAnkiAddedNotice(notice + 1));
    assert_eq!(Yomoku.view(&model).anki_added_notice, Some(notice));
    update(&mut model, Event::DismissAnkiAddedNotice(notice));
    update(&mut model, done);
    assert_eq!(Yomoku.view(&model).anki_added_notice, None);
    for result in [
        AnkiLiveResult::NoteExisting(vec![88]),
        AnkiLiveResult::NoteUncertain(None),
        AnkiLiveResult::NoteUncertain(Some(88)),
        AnkiLiveResult::NoSubmission,
        AnkiLiveResult::NoteAdded(-1),
    ] {
        let (mut model, _) = ready();
        let mut command = dispatch(&mut model);
        let done = callback(&mut command, result);
        update(&mut model, done.clone());
        update(&mut model, done);
        assert_eq!(Yomoku.view(&model).anki_added_notice, None);
    }
}

#[test]
fn confirmed_notice_renders_even_if_originating_export_is_no_longer_visible() {
    let (mut model, _) = ready();
    let mut command = dispatch(&mut model);
    let done = callback(&mut command, AnkiLiveResult::NoteAdded(88));
    model.prepared_anki_export = None;
    model.anki_entries.clear();
    let mut finished = Yomoku.update(done, &mut model);
    assert!(finished
        .effects()
        .any(|effect| matches!(effect, Effect::Render(_))));
    assert!(Yomoku.view(&model).anki_added_notice.is_some());
}
