use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, TextAnchor, TextRange},
    effective_reader::{EffectiveReaderChapter, EffectiveReaderTokenSnapshot},
    effective_session::{EffectiveReaderSelection, EffectiveReaderSession},
    reader::{ReaderAnalysis, ReaderTokenKey, ReaderVocabulary},
    vocabulary::{Surface, VocabularyStatus},
};

#[path = "effective_session/completions.rs"]
mod completions;
#[path = "effective_session/requests.rs"]
mod requests;
#[path = "effective_session/statuses.rs"]
mod statuses;
#[path = "effective_session/words.rs"]
mod words;

fn snapshot() -> EffectiveReaderTokenSnapshot {
    snapshot_with_separate_tokens(false)
}

fn snapshot_with_separate_tokens(separate: bool) -> EffectiveReaderTokenSnapshot {
    let chapter = ChapterText {
        source_id: 2.try_into().unwrap(),
        chapter_id: 3.try_into().unwrap(),
        text: "𠮷か\u{3099}\0。猫犬".into(),
    };
    let mut start = 0;
    let original = ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "fixture@1".into(),
            dictionary_revision: "full@1".into(),
        },
        mode: SplitMode::C,
        tokens: if separate {
            vec!["𠮷か\u{3099}\0", "。", "猫", "犬"]
        } else {
            vec!["𠮷か\u{3099}\0", "。", "猫犬"]
        }
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
                is_japanese: surface != "。",
                reading: if separate && ["猫", "犬"].contains(&surface) {
                    None
                } else {
                    Some(format!("読み{surface}"))
                },
                dictionary_form: if separate && ["猫", "犬"].contains(&surface) {
                    None
                } else {
                    Some(Surface::new(surface).unwrap())
                },
            };
            start = end;
            token
        })
        .collect(),
    };
    let resolved =
        EffectiveReaderChapter::resolve(&chapter, 4.try_into().unwrap(), &original).unwrap();
    // Clip both ends; selectable occurrences retain their complete identity.
    let page = TextAnchor {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        range: range(4, chapter.text.len() - 3),
    };
    EffectiveReaderTokenSnapshot {
        document_id: 1.try_into().unwrap(),
        page,
        tokens: resolved.tokens_on_page(page).unwrap(),
        segmentation: Some(resolved.into_segmentation()),
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: (start as i64).try_into().unwrap(),
        end: (end as i64).try_into().unwrap(),
    }
}

fn base(snapshot: &EffectiveReaderTokenSnapshot) -> ReaderTokenKey {
    ReaderTokenKey {
        document_id: snapshot.document_id,
        page: snapshot.page,
        analysis: snapshot
            .segmentation
            .as_ref()
            .map_or(ReaderAnalysis::Missing, |segmentation| {
                ReaderAnalysis::Available(segmentation.original_analysis_id)
            }),
    }
}
