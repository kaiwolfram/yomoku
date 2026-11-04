use super::super::test_support::fixture;
use super::*;
use crate::anki::{AnkiModelCreation, AnkiModelLayout, AnkiModelRecovery, AnkiRecoverySnapshot};
use crate::app::anki::test_support::reader_view;

fn prepare(model: &mut Model, selection: AnkiSelection, deck: i64) {
    assert!(Yomoku
        .update(
            Event::PrepareAnkiExport {
                selection,
                deck: deck.try_into().unwrap(),
            },
            model
        )
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
}

// Exercise acknowledged settings through their actual callback ownership, while
// preserving the reader route. A held Command represents an unanswered FIFO request.
fn start_settings(model: &mut Model, event: Event) -> Command<Effect, Event> {
    let was_open = model.settings_open;
    model.settings_open = true;
    let command = Yomoku.update(event, model);
    model.settings_open = was_open;
    command
}

fn finish_settings(model: &mut Model, mut command: Command<Effect, Event>, result: StorageResult) {
    let mut requests = command.effects().filter_map(|effect| match effect {
        Effect::Storage(request) => Some(request),
        Effect::Render(_) => None,
        _ => panic!("settings cannot start provider I/O"),
    });
    let mut request = requests.next().unwrap();
    assert!(requests.next().is_none());
    drop(requests);
    request.resolve(result).unwrap();
    for event in command.events() {
        assert!(Yomoku
            .update(event, model)
            .effects()
            .all(|effect| matches!(effect, Effect::Render(_))));
    }
}

fn configure(model: &mut Model, deck: Option<i64>) {
    let command = start_settings(model, Event::LoadAnkiSettings);
    finish_settings(
        model,
        command,
        StorageResult::AnkiSettingsLoaded(deck.map(|id| id.try_into().unwrap())),
    );
}

fn prepare_configured(model: &mut Model, selection: AnkiSelection) {
    assert!(Yomoku
        .update(Event::PrepareReaderAnkiExport { selection }, model)
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
}

#[test]
fn anki_preparation_freezes_current_original_and_effective_without_io_or_reader_changes() {
    for effective in [false, true] {
        let (mut model, selection) = fixture(effective);
        let before = reader_view(&model);
        prepare(&mut model, selection, i64::MAX);
        let export = model.prepared_anki_export().unwrap();
        assert_eq!(export.selection(), selection);
        assert_eq!(i64::from(export.deck()), i64::MAX);
        assert_eq!(export.layout(), &AnkiModelLayout::yomoku());
        assert_eq!(export.note().fields()[0], "猫");
        assert_eq!(export.note().fields()[1], "ねこ");
        assert_eq!(
            export.note().fields()[3],
            "猫と𠮷と<strong class=\"yomoku-target\">猫</strong>。"
        );
        assert_eq!(reader_view(&model), before);
    }
}

#[test]
fn anki_preparation_requires_successful_empty_recovery_and_no_commit_error() {
    for effective in [false, true] {
        for state in [
            AnkiRecoveryState::Unrestored,
            AnkiRecoveryState::Loading,
            AnkiRecoveryState::Failed(DocumentError::Unavailable),
            AnkiRecoveryState::Restored(Box::new(AnkiRecoverySnapshot {
                note: None,
                model: Some(AnkiModelRecovery {
                    id: 1.try_into().unwrap(),
                    creation: AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                    known_model_id: None,
                }),
            })),
        ] {
            let (mut model, selected) = fixture(effective);
            model.anki_recovery = state;
            prepare(&mut model, selected, 1);
            assert!(model.prepared_anki_export().is_none());
        }
        let (mut model, selected) = fixture(effective);
        model.anki_recovery_commit_error = Some(DocumentError::Unavailable);
        prepare(&mut model, selected, 1);
        assert!(model.prepared_anki_export().is_none());
    }
}

#[test]
fn anki_preparation_rejects_hidden_departed_busy_and_incomplete_dictionary_states() {
    let mutations: &[fn(&mut Model)] = &[
        |m| m.settings_open = true,
        |m| m.storage_busy = true,
        |m| m.content_route = ContentRoute::Main,
        |m| m.reader_document = None,
        |m| m.deferred_reader_event = Some(Event::CloseDocument),
        |m| m.clear_reader_word(),
        |m| m.reader_analysis = Some(ReaderAnalysis::Missing),
    ];
    for effective in [false, true] {
        for mutate in mutations {
            let (mut model, selection) = fixture(effective);
            mutate(&mut model);
            prepare(&mut model, selection, 1);
            assert!(model.prepared_anki_export().is_none());
        }
    }
}

#[test]
fn anki_preparation_rejects_obsolete_original_selection_and_modified_owner_metadata() {
    let mutations: &[fn(&mut Model)] = &[
        |m| m.reader_word_selection = None,
        |m| m.reader_word.as_mut().unwrap().token.reading = Some("stale".into()),
        |m| {
            m.reader_word.as_mut().unwrap().key.analysis =
                ReaderAnalysis::Available(99.try_into().unwrap())
        },
        |m| m.reader_word_error = Some(DocumentError::Unavailable),
        |m| m.pending_word_load = m.reader_word_selection,
        |m| m.reader_document.as_mut().unwrap().document_id = 99.try_into().unwrap(),
        |m| m.reader_document.as_mut().unwrap().chapters[0].text = "犬".into(),
        |m| m.reader_tokens = None,
    ];
    for mutate in mutations {
        let (mut model, selection) = fixture(false);
        mutate(&mut model);
        prepare(&mut model, selection, 1);
        assert!(model.prepared_anki_export().is_none());
    }
    let (mut model, AnkiSelection::Original(mut selection)) = fixture(false) else {
        unreachable!()
    };
    selection.token_index += 1;
    prepare(&mut model, AnkiSelection::Original(selection), 1);
    assert!(model.prepared_anki_export().is_none());
}

#[test]
fn anki_preparation_rejects_effective_revision_revisit_and_legacy_shell_selection() {
    let (mut model, AnkiSelection::Effective(selection)) = fixture(true) else {
        unreachable!()
    };
    let mut stale = selection;
    stale.page.revision += 1;
    prepare(&mut model, AnkiSelection::Effective(stale), 1);
    prepare(
        &mut model,
        AnkiSelection::Original(ReaderWordSelection {
            key: selection.page.base,
            token_index: selection.segment_index,
        }),
        1,
    );
    assert!(model.prepared_anki_export().is_none());
    let session = model.effective_reader.as_mut().unwrap();
    // An identical page revisit still retires the old selection and revision.
    session.begin_load(selection.page.base).unwrap();
    prepare(&mut model, AnkiSelection::Effective(selection), 1);
    assert!(model.prepared_anki_export().is_none());
}

#[test]
fn anki_preparation_survives_dismissal_navigation_and_duplicate_events_until_explicit_clear() {
    for effective in [false, true] {
        let (mut model, selected) = fixture(effective);
        prepare(&mut model, selected, 7);
        let frozen = model.prepared_anki_export().unwrap().clone();
        prepare(&mut model, selected, 8);
        assert_eq!(model.prepared_anki_export(), Some(&frozen));
        model.clear_reader_word();
        model.close_document();
        prepare(&mut model, selected, 9);
        assert_eq!(model.prepared_anki_export(), Some(&frozen));
        assert!(Yomoku
            .update(Event::ClearPreparedAnkiExport, &mut model)
            .effects()
            .all(|effect| matches!(effect, Effect::Render(_))));
        assert!(model.prepared_anki_export().is_none());
        assert_eq!(
            model.anki_recovery,
            AnkiRecoveryState::Restored(Box::default())
        );
        assert!(Model::default().prepared_anki_export().is_none());
    }
}

#[test]
fn anki_preparation_and_clear_cannot_erase_note_recovery_or_a_pending_commit() {
    use crate::anki::{
        AnkiReconciliation, AnkiRecovery, AnkiRecoveryCommit, AnkiRecoveryObservation,
        AnkiSubmission,
    };
    for effective in [false, true] {
        let (mut model, selected) = fixture(effective);
        prepare(&mut model, selected, 1);
        let export = model.prepared_anki_export().unwrap();
        let snapshot = Box::new(AnkiRecoverySnapshot {
            note: Some(AnkiRecovery {
                id: 1.try_into().unwrap(),
                submission: AnkiSubmission::new(
                    export.note().clone(),
                    export.deck(),
                    2.try_into().unwrap(),
                    export.layout().clone(),
                )
                .unwrap(),
                known_note_id: Some(9007199254740993.try_into().unwrap()),
            }),
            model: None,
        });
        model.anki_recovery = AnkiRecoveryState::Restored(snapshot.clone());
        assert!(Yomoku
            .update(Event::ClearPreparedAnkiExport, &mut model)
            .effects()
            .all(|effect| matches!(effect, Effect::Render(_))));
        prepare(&mut model, selected, 1);
        assert!(model.prepared_anki_export().is_none());
        assert_eq!(
            model.anki_recovery,
            AnkiRecoveryState::Restored(snapshot.clone())
        );
        let commit = Box::new(AnkiRecoveryCommit {
            expected: snapshot,
            observation: AnkiRecoveryObservation::Note(AnkiReconciliation::Unresolved),
        });
        model.anki_recovery_commit = Some((1, commit.clone()));
        prepare(&mut model, selected, 1);
        assert!(model.prepared_anki_export().is_none());
        assert_eq!(model.anki_recovery_commit.as_ref().unwrap().1, commit);
    }
}

#[test]
fn anki_shell_preparation_uses_configured_deck_and_never_reuses_generations() {
    for effective in [false, true] {
        let (mut model, selection) = fixture(effective);
        configure(&mut model, Some(i64::MAX));
        let event = Event::PrepareReaderAnkiExport { selection };
        let _prepare = Yomoku.update(event.clone(), &mut model);
        let first = Yomoku.view(&model).anki_export.unwrap();
        assert_eq!(first.preparation_id, 1);
        let _duplicate = Yomoku.update(event.clone(), &mut model);
        assert_eq!(Yomoku.view(&model).anki_export, Some(first.clone()));
        let _foreign_clear =
            Yomoku.update(Event::ClearAnkiExport { preparation_id: 0 }, &mut model);
        assert_eq!(Yomoku.view(&model).anki_export, Some(first.clone()));
        let _clear = Yomoku.update(
            Event::ClearAnkiExport {
                preparation_id: first.preparation_id,
            },
            &mut model,
        );
        assert!(model.prepared_anki_export().is_none());
        let _prepare = Yomoku.update(event.clone(), &mut model);
        let second = Yomoku.view(&model).anki_export.unwrap();
        assert_eq!(second.preparation_id, 2);
        let _stale_clear = Yomoku.update(
            Event::ClearAnkiExport {
                preparation_id: first.preparation_id,
            },
            &mut model,
        );
        assert_eq!(Yomoku.view(&model).anki_export, Some(second));
        let _clear = Yomoku.update(Event::ClearAnkiExport { preparation_id: 2 }, &mut model);
        model.anki_preparation_sequence = u64::MAX;
        let _exhausted = Yomoku.update(event, &mut model);
        assert!(model.prepared_anki_export().is_none());
    }
}

#[test]
fn anki_shell_preparation_cannot_select_hidden_stale_or_unloaded_dictionary_data() {
    for effective in [false, true] {
        for mutate in [
            (|m: &mut Model| m.clear_reader_word()) as fn(&mut Model),
            |m| m.settings_open = true,
            |m| m.storage_busy = true,
            |m| m.anki_recovery = AnkiRecoveryState::Unrestored,
            |m| m.close_document(),
        ] {
            let (mut model, selection) = fixture(effective);
            configure(&mut model, Some(1));
            mutate(&mut model);
            assert!(Yomoku
                .update(Event::PrepareReaderAnkiExport { selection }, &mut model)
                .effects()
                .next()
                .is_none());
            assert!(model.prepared_anki_export().is_none());
        }
        let (mut model, selection) = fixture(effective);
        configure(&mut model, Some(1));
        let stale = match selection {
            AnkiSelection::Original(mut value) => {
                value.token_index += 1;
                AnkiSelection::Original(value)
            }
            AnkiSelection::Effective(mut value) => {
                value.page.revision += 1;
                AnkiSelection::Effective(value)
            }
        };
        let _stale = Yomoku.update(
            Event::PrepareReaderAnkiExport { selection: stale },
            &mut model,
        );
        assert!(model.prepared_anki_export().is_none());
    }
}

#[test]
fn anki_configured_preparation_requires_ready_valid_configured_settings_without_io() {
    for effective in [false, true] {
        for phase in [
            AnkiSettingsPhase::Unloaded,
            AnkiSettingsPhase::Loading,
            AnkiSettingsPhase::Saving,
            AnkiSettingsPhase::Failed,
        ] {
            let (mut model, selection) = fixture(effective);
            model.anki_settings.phase = phase;
            model.anki_settings.default_deck_id = Some(i64::MAX);
            let before = Yomoku.view(&model);
            prepare_configured(&mut model, selection);
            assert_eq!(Yomoku.view(&model), before);
            assert!(model.prepared_anki_export().is_none());
        }
        for deck in [None, Some(0), Some(-1), Some(i64::MIN)] {
            let (mut model, selection) = fixture(effective);
            model.anki_settings.phase = AnkiSettingsPhase::Ready;
            model.anki_settings.default_deck_id = deck;
            prepare_configured(&mut model, selection);
            assert!(model.prepared_anki_export().is_none());
        }
    }
}

#[test]
fn anki_configured_preparation_waits_for_acknowledgement_and_freezes_exact_fields_and_deck() {
    for effective in [false, true] {
        for deck in [1, 9007199254740993, i64::MAX] {
            let (mut model, selection) = fixture(effective);
            configure(&mut model, None);
            let command = start_settings(&mut model, Event::SetAnkiDefaultDeck(Some(deck)));
            prepare_configured(&mut model, selection);
            assert!(model.prepared_anki_export().is_none());
            finish_settings(
                &mut model,
                command,
                StorageResult::AnkiDefaultDeckSaved(Some(deck.try_into().unwrap())),
            );
            let before = reader_view(&model);
            prepare_configured(&mut model, selection);
            let frozen = model.prepared_anki_export().unwrap().clone();
            assert_eq!(i64::from(frozen.deck()), deck);
            assert_eq!(frozen.selection(), selection);
            assert_eq!(frozen.note().fields()[0], "猫");
            assert_eq!(frozen.note().fields()[1], "ねこ");
            assert_eq!(
                frozen.note().fields()[3],
                "猫と𠮷と<strong class=\"yomoku-target\">猫</strong>。"
            );
            assert_eq!(reader_view(&model), before);
            let command = start_settings(&mut model, Event::SetAnkiDefaultDeck(None));
            finish_settings(
                &mut model,
                command,
                StorageResult::AnkiDefaultDeckSaved(None),
            );
            prepare_configured(&mut model, selection);
            assert_eq!(model.prepared_anki_export(), Some(&frozen));
            drop(Yomoku.update(Event::ClearPreparedAnkiExport, &mut model));
            prepare_configured(&mut model, selection);
            assert!(model.prepared_anki_export().is_none());
        }
    }
}

#[test]
fn anki_configured_preparation_rejects_failed_settings_with_old_acknowledged_deck() {
    for effective in [false, true] {
        for result in [
            StorageResult::Failed(DocumentError::Unavailable),
            StorageResult::AnkiDefaultDeckSaved(None),
            StorageResult::AnkiSettingsLoaded(None),
        ] {
            let (mut model, selection) = fixture(effective);
            configure(&mut model, Some(1));
            let command = start_settings(&mut model, Event::SetAnkiDefaultDeck(Some(i64::MAX)));
            finish_settings(&mut model, command, result);
            assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Failed);
            assert_eq!(model.anki_settings.default_deck_id, Some(1));
            prepare_configured(&mut model, selection);
            assert!(model.prepared_anki_export().is_none());
            let command = start_settings(&mut model, Event::RetryAnkiSettings);
            prepare_configured(&mut model, selection);
            assert!(model.prepared_anki_export().is_none());
            finish_settings(
                &mut model,
                command,
                StorageResult::AnkiSettingsLoaded(Some(1.try_into().unwrap())),
            );
            prepare_configured(&mut model, selection);
            assert_eq!(i64::from(model.prepared_anki_export().unwrap().deck()), 1);
        }
    }
}

#[test]
fn anki_configured_preparation_uses_authoritative_readback_after_lost_save_response() {
    for effective in [false, true] {
        let (mut model, selection) = fixture(effective);
        configure(&mut model, Some(1));
        let save = start_settings(&mut model, Event::SetAnkiDefaultDeck(Some(i64::MAX)));
        let read = start_settings(&mut model, Event::RetryAnkiSettings);
        finish_settings(
            &mut model,
            save,
            StorageResult::AnkiDefaultDeckSaved(Some(i64::MAX.try_into().unwrap())),
        );
        assert_eq!(model.anki_settings.phase, AnkiSettingsPhase::Loading);
        prepare_configured(&mut model, selection);
        assert!(model.prepared_anki_export().is_none());
        finish_settings(
            &mut model,
            read,
            StorageResult::AnkiSettingsLoaded(Some(9007199254740993.try_into().unwrap())),
        );
        prepare_configured(&mut model, selection);
        assert_eq!(
            i64::from(model.prepared_anki_export().unwrap().deck()),
            9007199254740993
        );
    }
}
