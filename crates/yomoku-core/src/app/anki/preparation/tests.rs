use super::*;
use crate::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    anki::{AnkiModelCreation, AnkiModelLayout, AnkiModelRecovery, AnkiRecoverySnapshot},
    dictionary::DictionaryCandidates,
    document::{ChapterText, TextOffset, TextRange},
    effective_reader::{
        EffectiveReaderChapter, EffectiveReaderTokenSnapshot, EffectiveReaderWordDetails,
    },
    effective_storage::EffectiveStorageResult,
    reader::{reader_tokens_on_page, ReaderVocabulary},
    vocabulary::VocabularyStatus,
};

fn fixture(effective: bool) -> (Model, AnkiSelection) {
    let chapter = ChapterText {
        source_id: 1.try_into().unwrap(),
        chapter_id: 2.try_into().unwrap(),
        text: "猫と𠮷と猫。".into(),
    };
    let document = ReaderDocument {
        document_id: 3.try_into().unwrap(),
        title: "Anki current owner".into(),
        finished_at_least_once: false,
        reading_position: None,
        chapters: vec![chapter.clone()],
        formatting: vec![],
        images: vec![],
        source_ruby: vec![],
    };
    let page = ReaderPage::open_document(&document, PageLength::default()).unwrap();
    let analysis_id = 4.try_into().unwrap();
    let start = chapter.text.rfind('猫').unwrap();
    let original = ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "test@1".into(),
            dictionary_revision: "test@1".into(),
        },
        mode: SplitMode::B,
        tokens: vec![
            AnalyzedToken {
                anchor: TextAnchor {
                    range: TextRange {
                        start: TextOffset::START,
                        end: TextOffset::at(&chapter.text, start).unwrap(),
                    },
                    ..page.anchor
                },
                surface: chapter.text[..start].into(),
                is_japanese: true,
                reading: None,
                dictionary_form: None,
            },
            AnalyzedToken {
                anchor: TextAnchor {
                    range: TextRange {
                        start: TextOffset::at(&chapter.text, start).unwrap(),
                        end: TextOffset::at(&chapter.text, start + "猫".len()).unwrap(),
                    },
                    ..page.anchor
                },
                surface: "猫".into(),
                is_japanese: true,
                reading: Some("ねこ".into()),
                dictionary_form: None,
            },
            AnalyzedToken {
                anchor: TextAnchor {
                    range: TextRange {
                        start: TextOffset::at(&chapter.text, start + "猫".len()).unwrap(),
                        end: page.anchor.range.end,
                    },
                    ..page.anchor
                },
                surface: "。".into(),
                is_japanese: false,
                reading: None,
                dictionary_form: None,
            },
        ],
    };
    let vocabulary = Some(ReaderVocabulary {
        id: 5.try_into().unwrap(),
        status: VocabularyStatus::Learning,
    });
    let candidates = DictionaryCandidates {
        surface: "猫".into(),
        dictionary_form: None,
        words: vec![],
        names: vec![],
    };
    let mut model = Model {
        anki_recovery: AnkiRecoveryState::Restored(Box::default()),
        content_route: ContentRoute::Reader,
        reader: Some(page.clone()),
        reader_document: Some(document),
        reader_analysis: Some(ReaderAnalysis::Available(analysis_id)),
        ..if effective {
            Model::with_effective_reader()
        } else {
            Model::default()
        }
    };
    let selection = if effective {
        let session = model.effective_reader.as_mut().unwrap();
        let request = session
            .begin_load(ReaderTokenKey {
                document_id: page.document_id,
                page: page.anchor,
                analysis: ReaderAnalysis::Available(analysis_id),
            })
            .unwrap();
        let resolved =
            EffectiveReaderChapter::resolve(&chapter, analysis_id, &original, &[], &[]).unwrap();
        let mut tokens = resolved.tokens_on_page(page.anchor).unwrap();
        tokens[1].vocabulary = vocabulary;
        let snapshot = EffectiveReaderTokenSnapshot {
            document_id: page.document_id,
            page: page.anchor,
            segmentation: Some(resolved.into_segmentation()),
            tokens,
            original_boundaries: vec![],
        };
        let details = EffectiveReaderWordDetails {
            key: snapshot.key().unwrap(),
            token: snapshot.tokens[1].clone(),
            encountered: 47,
            candidates,
            custom_entry: None,
        };
        assert!(session.accept(request, snapshot));
        let selected = EffectiveReaderSelection {
            page: session.page().unwrap().identity,
            segment_index: session.page().unwrap().tokens[1].segment_index,
        };
        let request = session.begin_word_load(selected).unwrap();
        assert!(session.finish_word_request(
            &request,
            Ok(EffectiveStorageResult::WordLoaded(
                selected,
                Box::new(details)
            ))
        ));
        AnkiSelection::Effective(selected)
    } else {
        let mut tokens = reader_tokens_on_page(&chapter, &original, page.anchor).unwrap();
        tokens[1].vocabulary = vocabulary;
        let snapshot = ReaderTokenSnapshot {
            document_id: page.document_id,
            page: page.anchor,
            analysis: ReaderAnalysis::Available(analysis_id),
            tokens,
        };
        let details = ReaderWordDetails {
            key: snapshot.key(),
            token: snapshot.tokens[1].clone(),
            encountered: 47,
            candidates,
            custom_entry: None,
        };
        let selected = details.selection();
        model.reader_word_selection = Some(selected);
        model.reader_word = Some(details);
        model.reader_tokens = Some(snapshot);
        AnkiSelection::Original(selected)
    };
    (model, selection)
}

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
        .next()
        .is_none());
}

#[test]
fn anki_preparation_freezes_current_original_and_effective_without_effects_or_reader_changes() {
    for effective in [false, true] {
        let (mut model, selection) = fixture(effective);
        let before = Yomoku.view(&model);
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
        assert_eq!(Yomoku.view(&model), before);
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
            .next()
            .is_none());
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
        let before = Yomoku.view(&model);
        prepare(&mut model, AnkiSelection::Effective(selected), 1);
        assert!(model.prepared_anki_export().is_none());
        assert_eq!(Yomoku.view(&model), before);
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
        .next()
        .is_none());
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
            .next()
            .is_none());
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
