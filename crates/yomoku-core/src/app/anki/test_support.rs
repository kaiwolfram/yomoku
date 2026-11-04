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
    reader::{reader_tokens_on_page, ReaderVocabulary},
    vocabulary::VocabularyStatus,
};

/// Compare reader state independently of the export presentation, which has
/// dedicated projection assertions.
pub(super) fn reader_view(model: &Model) -> ViewModel {
    let mut view = Yomoku.view(model);
    view.anki_export = None;
    view
}

pub(super) fn fixture(effective: bool) -> (Model, AnkiSelection) {
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
