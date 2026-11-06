//! Immutable longest-token segmentation with exact analyzer metadata.

use serde::Serialize;

use crate::{
    analysis::{AnalysisProfile, ChapterAnalysis, InvalidAnalysis, SplitMode},
    document::{AnalysisId, ChapterText, TextAnchor},
    vocabulary::Surface,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedSegment {
    pub anchor: TextAnchor,
    pub surface: String,
    /// Present only when this segment exactly matches an original Mode C token.
    /// Consumers may retrieve metadata from that immutable token, never guess it.
    pub original_token_index: Option<usize>,
}

/// Analyzer metadata exists only for an exact span in a validated snapshot.
/// Metadata retains its immutable analysis identity.
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
    pub boundary: ResolvedSegment,
    /// None means unresolved, including classification. It does not mean
    /// non-Japanese, and never licenses guessing a reading or dictionary form.
    pub metadata: Option<SegmentMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveChapterSegmentation {
    pub original_analysis_id: AnalysisId,
    pub profile: AnalysisProfile,
    pub segments: Vec<EffectiveSegment>,
}

/// Project the immutable longest-token analysis without user overrides.
pub fn resolve_effective_segmentation(
    chapter: &ChapterText,
    original_id: AnalysisId,
    original: &ChapterAnalysis,
) -> Result<EffectiveChapterSegmentation, InvalidAnalysis> {
    let boundaries = resolve_segmentation(chapter, original)?;
    Ok(EffectiveChapterSegmentation {
        original_analysis_id: original_id,
        profile: original.profile.clone(),
        segments: boundaries
            .into_iter()
            .enumerate()
            .map(|(index, boundary)| {
                let token = &original.tokens[index];
                EffectiveSegment {
                    boundary,
                    metadata: Some(SegmentMetadata {
                        analysis_id: original_id,
                        mode: original.mode,
                        token_index: index,
                        is_japanese: token.is_japanese,
                        reading: token.reading.clone(),
                        dictionary_form: token.dictionary_form.clone(),
                    }),
                }
            })
            .collect(),
    })
}

pub fn resolve_segmentation(
    chapter: &ChapterText,
    original: &ChapterAnalysis,
) -> Result<Vec<ResolvedSegment>, InvalidAnalysis> {
    original.validate(chapter)?;
    if original.mode != SplitMode::C {
        return Err(InvalidAnalysis);
    }
    Ok(original
        .tokens
        .iter()
        .enumerate()
        .map(|(index, token)| ResolvedSegment {
            anchor: token.anchor,
            surface: token.surface.clone(),
            original_token_index: Some(index),
        })
        .collect())
}
