//! Immutable Mode C segmentation with mandatory exact analyzer metadata.

use serde::Serialize;

use crate::{
    analysis::{AnalysisProfile, ChapterAnalysis, InvalidAnalysis, SplitMode},
    document::{AnalysisId, ChapterText, TextAnchor},
    vocabulary::Surface,
};

/// Identity fields are retained in serialized completion occurrences for the
/// existing version-2 reservation format. Readings and base forms remain optional.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SegmentMetadata {
    pub analysis_id: AnalysisId,
    pub mode: SplitMode,
    pub token_index: usize,
    pub is_japanese: bool,
    pub reading: Option<String>,
    pub dictionary_form: Option<Surface>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveSegment {
    pub anchor: TextAnchor,
    pub surface: String,
    pub metadata: SegmentMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveChapterSegmentation {
    pub original_analysis_id: AnalysisId,
    pub profile: AnalysisProfile,
    pub segments: Vec<EffectiveSegment>,
}

/// Project validated immutable Mode C directly, without intermediate boundaries.
pub fn resolve_effective_segmentation(
    chapter: &ChapterText,
    original_id: AnalysisId,
    original: &ChapterAnalysis,
) -> Result<EffectiveChapterSegmentation, InvalidAnalysis> {
    original.validate(chapter)?;
    if original.mode != SplitMode::C {
        return Err(InvalidAnalysis);
    }
    Ok(EffectiveChapterSegmentation {
        original_analysis_id: original_id,
        profile: original.profile.clone(),
        segments: original
            .tokens
            .iter()
            .enumerate()
            .map(|(index, token)| EffectiveSegment {
                anchor: token.anchor,
                surface: token.surface.clone(),
                metadata: SegmentMetadata {
                    analysis_id: original_id,
                    mode: original.mode,
                    token_index: index,
                    is_japanese: token.is_japanese,
                    reading: token.reading.clone(),
                    dictionary_form: token.dictionary_form.clone(),
                },
            })
            .collect(),
    })
}
