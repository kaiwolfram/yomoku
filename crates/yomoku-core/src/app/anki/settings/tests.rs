use super::*;
use crate::anki::AnkiSettingsView;

fn request(model: &mut Model, event: Event) -> StorageOperation {
    let mut command = Yomoku.update(event, model);
    let mut operations = command.effects().filter_map(|effect| match effect {
        Effect::Storage(request) => Some(request.operation),
        Effect::Render(_) => None,
        _ => panic!("local settings cannot dispatch provider operations"),
    });
    let operation = operations.next().expect("one local operation");
    assert!(operations.next().is_none());
    operation
}

fn finish(model: &mut Model, id: u64, result: StorageResult) {
    let effects: Vec<_> = Yomoku
        .update(Event::AnkiSettingsFinished(id, result), model)
        .effects()
        .collect();
    assert!(effects
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
}

fn quiet(model: &mut Model, event: Event) {
    assert!(Yomoku.update(event, model).effects().next().is_none());
}

fn loaded(deck: Option<i64>) -> Model {
    let mut model = Model {
        settings_open: true,
        ..Model::default()
    };
    assert_eq!(
        request(&mut model, Event::LoadAnkiSettings),
        StorageOperation::LoadAnkiSettings
    );
    finish(
        &mut model,
        1,
        StorageResult::AnkiSettingsLoaded(deck.map(|id| id.try_into().unwrap())),
    );
    model
}

#[test]
fn anki_settings_lazy_load_distinguishes_unloaded_from_unconfigured_and_retains_on_navigation() {
    let mut model = Model::default();
    assert_eq!(
        Yomoku.view(&model).anki_settings,
        AnkiSettingsView::default()
    );
    for event in [
        Event::LoadAnkiSettings,
        Event::RetryAnkiSettings,
        Event::SetAnkiDefaultDeck(Some(1)),
    ] {
        quiet(&mut model, event);
    }
    let mut navigation = Yomoku.update(Event::OpenSettings, &mut model);
    assert!(navigation
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
    assert_eq!(
        request(&mut model, Event::LoadAnkiSettings),
        StorageOperation::LoadAnkiSettings
    );
    quiet(&mut model, Event::LoadAnkiSettings);
    quiet(&mut model, Event::SetAnkiDefaultDeck(Some(1)));
    finish(&mut model, 1, StorageResult::AnkiSettingsLoaded(None));
    assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
    assert_eq!(model.anki_settings.default_deck_id, None);
    drop(Yomoku.update(Event::CloseSettings, &mut model));
    drop(Yomoku.update(Event::OpenSettings, &mut model));
    quiet(&mut model, Event::LoadAnkiSettings);
    quiet(&mut model, Event::SetAnkiDefaultDeck(None));
    quiet(&mut model, Event::RetryAnkiSettings);
    assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
}

#[test]
fn anki_settings_save_and_clear_publish_only_matching_acknowledgements() {
    let mut model = loaded(None);
    for id in [i64::MIN, -1, 0] {
        quiet(&mut model, Event::SetAnkiDefaultDeck(Some(id)));
    }
    for id in [1, 9007199254740993, i64::MAX] {
        let sequence = model.anki_settings_sequence + 1;
        let deck = Some(AnkiDeckId::try_from(id).unwrap());
        assert_eq!(
            request(&mut model, Event::SetAnkiDefaultDeck(Some(id))),
            StorageOperation::SaveAnkiDefaultDeck(deck)
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Saving);
        assert_ne!(model.anki_settings.default_deck_id, Some(id));
        quiet(&mut model, Event::SetAnkiDefaultDeck(None));
        quiet(&mut model, Event::SetAnkiDefaultDeck(Some(id)));
        finish(
            &mut model,
            sequence - 1,
            StorageResult::AnkiDefaultDeckSaved(None),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Saving);
        // Navigation does not cancel an already accepted local save.
        model.settings_open = false;
        finish(
            &mut model,
            sequence,
            StorageResult::AnkiDefaultDeckSaved(deck),
        );
        assert_eq!(model.anki_settings.default_deck_id, Some(id));
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
        model.settings_open = true;
        quiet(&mut model, Event::SetAnkiDefaultDeck(Some(id)));
        finish(
            &mut model,
            sequence,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
    }
    let sequence = model.anki_settings_sequence + 1;
    assert_eq!(
        request(&mut model, Event::SetAnkiDefaultDeck(None)),
        StorageOperation::SaveAnkiDefaultDeck(None)
    );
    assert_eq!(model.anki_settings.default_deck_id, Some(i64::MAX));
    finish(
        &mut model,
        sequence,
        StorageResult::AnkiDefaultDeckSaved(None),
    );
    assert_eq!(model.anki_settings.default_deck_id, None);
}

#[test]
fn anki_settings_failed_or_wrong_results_require_readback_before_another_save() {
    for result in [
        StorageResult::Failed(DocumentError::IncompatibleDatabase),
        StorageResult::AnkiDefaultDeckSaved(None),
        StorageResult::AnkiSettingsLoaded(Some(2.try_into().unwrap())),
        StorageResult::ImportCancelled,
    ] {
        let mut model = loaded(Some(1));
        request(&mut model, Event::SetAnkiDefaultDeck(Some(2)));
        finish(&mut model, 2, result);
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Failed);
        assert!(model.anki_settings.storage_error.is_some());
        assert_eq!(model.anki_settings.default_deck_id, Some(1));
        quiet(&mut model, Event::LoadAnkiSettings);
        quiet(&mut model, Event::SetAnkiDefaultDeck(Some(2)));
        assert_eq!(
            request(&mut model, Event::RetryAnkiSettings),
            StorageOperation::LoadAnkiSettings
        );
        finish(
            &mut model,
            3,
            StorageResult::AnkiSettingsLoaded(Some(2.try_into().unwrap())),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
        assert_eq!(model.anki_settings.storage_error, None);
        quiet(&mut model, Event::SetAnkiDefaultDeck(Some(2)));
    }
    let mut model = Model {
        settings_open: true,
        ..Model::default()
    };
    request(&mut model, Event::LoadAnkiSettings);
    finish(&mut model, 1, StorageResult::AnkiDefaultDeckSaved(None));
    assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Failed);
    assert_eq!(model.anki_settings.default_deck_id, None);
}

#[test]
fn anki_settings_lost_reads_and_saves_retry_readonly_and_reject_stale_callbacks() {
    for save in [false, true] {
        let mut model = loaded(None);
        if save {
            request(&mut model, Event::SetAnkiDefaultDeck(Some(i64::MAX)));
        } else {
            model.anki_settings.phase = AnkiSettingsPhase::Unloaded;
            request(&mut model, Event::LoadAnkiSettings);
        }
        let old_id = model.anki_settings_sequence;
        assert_eq!(
            request(&mut model, Event::RetryAnkiSettings),
            StorageOperation::LoadAnkiSettings
        );
        let current = model.anki_settings_sequence;
        finish(
            &mut model,
            old_id,
            StorageResult::AnkiDefaultDeckSaved(Some(i64::MAX.try_into().unwrap())),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Loading);
        finish(
            &mut model,
            current,
            StorageResult::AnkiSettingsLoaded(Some(i64::MAX.try_into().unwrap())),
        );
        finish(
            &mut model,
            old_id,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Ready);
        assert_eq!(model.anki_settings.default_deck_id, Some(i64::MAX));
    }
}

#[test]
fn anki_settings_generation_exhaustion_invalidates_old_responses_without_dispatch() {
    let mut model = loaded(Some(1));
    request(&mut model, Event::SetAnkiDefaultDeck(None));
    model.anki_settings_sequence = u64::MAX;
    assert!(Yomoku
        .update(Event::RetryAnkiSettings, &mut model)
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
    finish(&mut model, 2, StorageResult::AnkiDefaultDeckSaved(None));
    assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Failed);
    assert_eq!(model.anki_settings.default_deck_id, Some(1));
    assert_eq!(model.anki_settings_request.map(|value| value.0), None);
}

#[test]
fn anki_settings_callbacks_preserve_reader_and_frozen_export_for_both_selection_kinds() {
    for effective in [false, true] {
        let (mut model, selection) = super::super::test_support::fixture(effective);
        drop(Yomoku.update(
            Event::PrepareReaderAnkiExport {
                selection,
                deck_id: 1,
            },
            &mut model,
        ));
        assert!(model.prepared_anki_export.is_some());
        model.settings_open = true;
        model.storage_busy = true;
        model.document_error = Some(DocumentError::MissingAnalysis);
        let before = Yomoku.view(&model);
        let recovery = model.anki_recovery.clone();
        let export = model.prepared_anki_export.clone();
        let mut command = Yomoku.update(Event::LoadAnkiSettings, &mut model);
        let mut effects = command.effects();
        let Effect::Storage(mut storage_request) = effects
            .find(|effect| matches!(effect, Effect::Storage(_)))
            .unwrap()
        else {
            unreachable!()
        };
        drop(effects);
        storage_request
            .resolve(StorageResult::AnkiSettingsLoaded(Some(
                2.try_into().unwrap(),
            )))
            .unwrap();
        let event = command.events().next().unwrap();
        assert!(matches!(event, Event::AnkiSettingsFinished(1, _)));
        drop(Yomoku.update(event, &mut model));
        request(&mut model, Event::SetAnkiDefaultDeck(None));
        finish(&mut model, 2, StorageResult::AnkiDefaultDeckSaved(None));
        let mut after = Yomoku.view(&model);
        after.anki_settings = before.anki_settings.clone();
        assert_eq!(after, before);
        assert_eq!(model.anki_recovery, recovery);
        assert_eq!(model.prepared_anki_export, export);
        assert_eq!(
            Yomoku.view(&Model::default()).anki_settings.phase,
            AnkiSettingsPhase::Unloaded
        );
    }
}
