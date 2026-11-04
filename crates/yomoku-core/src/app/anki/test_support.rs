use super::*;
use crate::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    anki::AnkiSelection,
    dictionary::DictionaryCandidates,
    document::{ChapterText, TextOffset, TextRange},
    effective_reader::{
        EffectiveReaderChapter, EffectiveReaderTokenSnapshot, EffectiveReaderWordDetails,
    },
    effective_storage::EffectiveStorageResult,
    reader::ReaderVocabulary,
    vocabulary::VocabularyStatus,
};

pub(super) fn fixture() -> (Model, AnkiSelection) {
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
        mode: SplitMode::C,
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
        content_route: ContentRoute::Reader,
        reader: Some(page.clone()),
        reader_document: Some(document),
        reader_analysis: Some(ReaderAnalysis::Available(analysis_id)),
        ..Model::default()
    };
    let selection = {
        let session = &mut model.effective_reader;
        let request = session
            .begin_load(ReaderTokenKey {
                document_id: page.document_id,
                page: page.anchor,
                analysis: ReaderAnalysis::Available(analysis_id),
            })
            .unwrap();
        let resolved = EffectiveReaderChapter::resolve(&chapter, analysis_id, &original).unwrap();
        let mut tokens = resolved.tokens_on_page(page.anchor).unwrap();
        tokens[1].vocabulary = vocabulary;
        let snapshot = EffectiveReaderTokenSnapshot {
            document_id: page.document_id,
            page: page.anchor,
            segmentation: Some(resolved.into_segmentation()),
            tokens,
        };
        let details = EffectiveReaderWordDetails {
            key: snapshot.key().unwrap(),
            token: snapshot.tokens[1].clone(),
            encountered: 47,
            candidates,

            dictionary_sources: Vec::new(),
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
    };
    (model, selection)
}

pub(super) fn set_candidates(model: &mut Model, words: Vec<crate::dictionary::WordCandidate>) {
    let session = &mut model.effective_reader;
    let current = session.word_state().details.unwrap();
    let command = session.freeze_selection(current.selection).unwrap();
    let metadata = command.key().segmentation.segments[current.selection.segment_index as usize]
        .metadata
        .clone();
    let details = EffectiveReaderWordDetails {
        key: command.key().clone(),
        token: crate::effective_reader::EffectiveReaderToken {
            segment_index: current.token.segment_index,
            anchor: current.token.anchor,
            display_range: current.token.display_range,
            surface: current.token.surface,
            eligibility: current.token.eligibility,
            metadata,
            vocabulary: current.token.vocabulary,
        },
        encountered: current.encountered,
        candidates: DictionaryCandidates {
            words,
            ..current.candidates
        },
        dictionary_sources: current.dictionary_sources,
    };
    let request = session.begin_word_load(current.selection).unwrap();
    assert!(session.finish_word_request(
        &request,
        Ok(EffectiveStorageResult::WordLoaded(
            current.selection,
            Box::new(details)
        ))
    ));
}
