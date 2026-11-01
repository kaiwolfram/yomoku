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
        |m| m.pending_custom_change = m.reader_word_selection,
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
fn anki_preparation_rejects_pending_or_failed_effective_dictionary_mutations() {
    for fail in [false, true] {
        let (mut model, AnkiSelection::Effective(selected)) = fixture(true) else {
            unreachable!()
        };
        let session = model.effective_reader.as_mut().unwrap();
        let request = session
            .begin_custom_entry_save(selected, "cat".into(), "ねこ".into())
            .unwrap();
        if fail {
            assert!(session.finish_word_request(&request, Err(DocumentError::Unavailable)));
        }
        let before = reader_view(&model);
        prepare(&mut model, AnkiSelection::Effective(selected), 1);
        assert!(model.prepared_anki_export().is_none());
        assert_eq!(reader_view(&model), before);
    }
    let (mut model, AnkiSelection::Effective(selected)) = fixture(true) else {
        unreachable!()
    };
    model
        .effective_reader
        .as_mut()
        .unwrap()
        .begin_word_load(selected)
        .unwrap();
    prepare(&mut model, AnkiSelection::Effective(selected), 1);
    assert!(model.prepared_anki_export().is_none());
}

#[test]
fn anki_preparation_keeps_frozen_definitions_until_clear_then_accepts_current_data() {
    let (mut model, selection) = fixture(false);
    prepare(&mut model, selection, 7);
    let frozen = model.prepared_anki_export().unwrap().clone();
    model.reader_word.as_mut().unwrap().custom_entry = Some(crate::reader::ReaderCustomEntry {
        surface: "猫".into(),
        english_meaning: "new & literal definition".into(),
        reading: "changed reading".into(),
    });
    prepare(&mut model, selection, 8);
    assert_eq!(model.prepared_anki_export(), Some(&frozen));
    assert!(Yomoku
        .update(Event::ClearPreparedAnkiExport, &mut model)
        .effects()
        .all(|effect| matches!(effect, Effect::Render(_))));
    prepare(&mut model, selection, 8);
    let current = model.prepared_anki_export().unwrap();
    assert_eq!(i64::from(current.deck()), 8);
    assert_eq!(current.note().fields()[1], "changed reading");
    assert_eq!(
        current.note().fields()[2],
        "<ol><li>new &amp; literal definition</li></ol>"
    );
    assert_eq!(current.note().fields()[3], frozen.note().fields()[3]);
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
