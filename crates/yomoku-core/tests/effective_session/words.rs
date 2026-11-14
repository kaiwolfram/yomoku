use super::*;
use yomoku_core::{
    dictionary::DictionaryCandidates,
    effective_reader::EffectiveReaderWordDetails,
    effective_session::EffectiveWordRequest,
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    storage::DocumentError,
};

pub(super) fn loaded(
    manual: bool,
) -> (
    EffectiveReaderSession,
    EffectiveReaderSelection,
    EffectiveReaderWordDetails,
) {
    let snapshot = snapshot_with_manual_split(manual);
    let token = snapshot.tokens.last().unwrap().clone();
    let mut pages = EffectiveReaderSession::default();
    let request = pages.begin_load(base(&snapshot)).unwrap();
    let details = EffectiveReaderWordDetails {
        key: snapshot.key().unwrap(),
        candidates: DictionaryCandidates {
            surface: token.surface.clone(),
            dictionary_form: token.metadata.as_ref().and_then(|metadata| {
                metadata
                    .dictionary_form
                    .as_ref()
                    .map(Surface::as_str)
                    .map(str::to_owned)
            }),
            words: vec![],
            names: vec![],
        },
        token,
        encountered: 7,

        dictionary_sources: Vec::new(),
    };
    assert!(pages.accept(request, snapshot));
    let selection = EffectiveReaderSelection {
        page: pages.page().unwrap().identity,
        segment_index: details.token.segment_index,
    };
    (pages, selection, details)
}

fn finish_load(
    pages: &mut EffectiveReaderSession,
    selection: EffectiveReaderSelection,
    details: EffectiveReaderWordDetails,
) -> EffectiveWordRequest {
    let request = pages.begin_word_load(selection).unwrap();
    assert!(matches!(request.operation(), Op::LoadWord(_)));
    assert!(pages.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    assert!(!pages.word_state().busy);
    request
}

#[test]
fn word_projection_is_compact_and_preserves_resolved_unresolved_and_live_fields() {
    for manual in [false, true] {
        for status in [
            None,
            Some(VocabularyStatus::New),
            Some(VocabularyStatus::Learning),
            Some(VocabularyStatus::Familiar),
            Some(VocabularyStatus::Known),
            Some(VocabularyStatus::Ignored),
        ] {
            let (mut pages, selection, mut details) = loaded(manual);
            details.token.vocabulary = status.map(|status| ReaderVocabulary {
                id: 55.try_into().unwrap(),
                status,
            });
            let expected = details.clone();
            finish_load(&mut pages, selection, details);
            let word = pages.word_state().details.unwrap();
            assert_eq!(word.selection, selection);
            assert_eq!(word.token, expected.token.into());
            assert_eq!(word.encountered, 8);
            assert_eq!(word.candidates, expected.candidates);
            assert_eq!(word.token.reading.is_none(), manual);
            assert_eq!(word.token.dictionary_form.is_none(), manual);
            assert_eq!(
                pages.page().unwrap().tokens.last().unwrap().vocabulary,
                word.token.vocabulary
            );
        }
    }
}

#[test]
fn dismissed_refreshed_and_duplicate_word_callbacks_cannot_resurrect_selection() {
    let (mut pages, selection, details) = loaded(true);
    let request = pages.begin_word_load(selection).unwrap();
    assert!(pages.begin_word_load(selection).is_none());
    pages.dismiss_word();
    assert!(pages.word_state().busy);
    assert!(pages.finish_word_request(
        &request,
        Ok(Output::WordLoaded(selection, Box::new(details.clone())))
    ));
    assert!(pages.word_state().selection.is_none());
    assert!(pages.word_state().details.is_none());
    assert!(!pages.word_state().busy);
    let second = pages.begin_word_load(selection).unwrap();
    assert_ne!(second, request);
    assert!(!pages.finish_word_request(&request, Err(DocumentError::Unavailable)));
    assert!(pages.word_state().busy);
    pages.invalidate();
    assert!(pages.finish_word_request(
        &second,
        Ok(Output::WordLoaded(selection, Box::new(details)))
    ));
    assert!(pages.word_state().selection.is_none());
    assert!(pages.word_state().details.is_none());
    assert!(!pages.word_state().busy);
    let snapshot = snapshot_with_manual_split(true);
    let load = pages.begin_load(base(&snapshot)).unwrap();
    assert!(pages.accept(load, snapshot));
    assert!(pages.begin_word_load(selection).is_none());
    let fresh = EffectiveReaderSelection {
        page: pages.page().unwrap().identity,
        ..selection
    };
    let third = pages.begin_word_load(fresh).unwrap();
    assert!(!pages.finish_word_request(&second, Err(DocumentError::Unavailable)));
    assert!(pages.finish_word_request(&third, Err(DocumentError::Unavailable)));
    assert_eq!(pages.word_state().error, Some(DocumentError::Unavailable));
    assert!(!pages.word_state().busy);
    assert!(pages.begin_word_load(fresh).is_some()); // explicit lookup retry only
}

#[test]
fn foreign_word_results_fail_closed_without_consuming_another_request() {
    for case in 0..10 {
        let (mut pages, selection, mut details) = loaded(true);
        let request = pages.begin_word_load(selection).unwrap();
        let mut returned = selection;
        match case {
            0 => returned.page.revision += 1,
            1 => details.key.document_id = 99.try_into().unwrap(),
            2 => details.key.segmentation.segments[0]
                .boundary
                .surface
                .push('別'),
            3 => details.token.segment_index += 1,
            4 => details.token.surface.push('別'),
            5 => details.token.display_range = range(0, 1),
            6 => details.token.metadata = snapshot().tokens.last().unwrap().metadata.clone(),
            7 => details.candidates.surface.push('別'),
            8 => details.token.anchor.range = range(0, 1),
            9 => details.candidates.dictionary_form = Some("guessed".into()),
            _ => unreachable!(),
        }
        assert!(pages.finish_word_request(
            &request,
            Ok(Output::WordLoaded(returned, Box::new(details)))
        ));
        assert_eq!(
            pages.word_state().error,
            Some(DocumentError::InvalidDocument),
            "case {case}"
        );
        assert!(pages.word_state().details.is_none());
        let next = pages.begin_word_load(selection).unwrap();
        assert!(!pages.finish_word_request(&request, Err(DocumentError::Unavailable)));
        assert!(pages.finish_word_request(&next, Err(DocumentError::Unavailable)));
    }
}

#[test]
fn encounter_projection_recounts_exact_visible_surfaces_on_navigation_and_rereads() {
    let chapter = ChapterText {
        source_id: 2.try_into().unwrap(),
        chapter_id: 3.try_into().unwrap(),
        text: "食べた食べた食べる".into(),
    };
    let mut start = 0;
    let analysis = ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::C,
        tokens: ["食べた", "食べた", "食べる"]
            .into_iter()
            .map(|surface| {
                let end = start + surface.len();
                let token = AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: range(start, end),
                    },
                    surface: surface.into(),
                    is_japanese: true,
                    reading: None,
                    dictionary_form: Some(Surface::new("食べる").unwrap()),
                };
                start = end;
                token
            })
            .collect(),
    };
    let mut session = EffectiveReaderSession::default();
    // Forward, backward, and a reread each get a fresh page projection.
    for (start, end, visible) in [(0, 9, 1), (9, 27, 1), (0, 27, 2), (0, 9, 1)] {
        for saved in [0, 7, u64::MAX] {
            let resolved =
                EffectiveReaderChapter::resolve(&chapter, 4.try_into().unwrap(), &analysis)
                    .unwrap();
            let page = TextAnchor {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                range: range(start, end),
            };
            let snapshot = EffectiveReaderTokenSnapshot {
                document_id: 1.try_into().unwrap(),
                page,
                tokens: resolved.tokens_on_page(page).unwrap(),
                segmentation: Some(resolved.into_segmentation()),
            };
            let token = snapshot.tokens.first().unwrap().clone();
            let details = EffectiveReaderWordDetails {
                key: snapshot.key().unwrap(),
                token,
                encountered: saved,
                candidates: DictionaryCandidates {
                    surface: "食べた".into(),
                    dictionary_form: Some("食べる".into()),
                    words: vec![],
                    names: vec![],
                },
                dictionary_sources: vec![],
            };
            let request = session.begin_load(base(&snapshot)).unwrap();
            assert!(session.word_state().details.is_none());
            assert!(session.accept(request, snapshot));
            let selection = EffectiveReaderSelection {
                page: session.page().unwrap().identity,
                segment_index: details.token.segment_index,
            };
            for _ in 0..3 {
                finish_load(&mut session, selection, details.clone());
                for _ in 0..3 {
                    assert_eq!(
                        session.word_state().details.unwrap().encountered,
                        saved.saturating_add(visible)
                    );
                }
                session.dismiss_word();
            }
        }
    }
}
