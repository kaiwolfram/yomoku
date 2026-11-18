//! Read-only reader projections over validated effective segmentation.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    analysis::{ChapterAnalysis, InvalidAnalysis},
    completion::PageCompletion,
    dictionary::DictionaryCandidates,
    document::{AnalysisId, ChapterText, DocumentId, TextAnchor, TextOffset, TextRange},
    reader::ReaderVocabulary,
    segmentation::{
        resolve_effective_segmentation, EffectiveChapterSegmentation, EffectiveSegment,
        SegmentMetadata,
    },
};

/// Eligibility is supplied by the validated analyzer snapshot.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum EffectiveTokenEligibility {
    AnalyzerJapanese,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveReaderToken {
    /// Whole effective-chapter index, including gaps for non-Japanese segments.
    /// With immutable Mode C segmentation this is also the stored token index.
    pub segment_index: u64,
    pub anchor: TextAnchor,
    pub display_range: TextRange,
    pub surface: String,
    pub eligibility: EffectiveTokenEligibility,
    pub metadata: SegmentMetadata,
    pub vocabulary: Option<ReaderVocabulary>,
}

/// Page lookup with resolved provenance. Missing segmentation means absent
/// original C; lookup never requests analysis or creates vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderTokenSnapshot {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: Option<Arc<EffectiveChapterSegmentation>>,
    pub tokens: Vec<EffectiveReaderToken>,
}

/// Frozen page and complete resolved provenance, deliberately separate from an
/// original analyzer identity. Live vocabulary/status changes do not invalidate
/// it; changed boundaries or metadata do. Full keys remain native.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveReaderTokenKey {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: Arc<EffectiveChapterSegmentation>,
}

impl EffectiveReaderTokenSnapshot {
    /// Missing original C cannot produce a selectable effective identity.
    pub fn key(&self) -> Option<EffectiveReaderTokenKey> {
        self.segmentation
            .as_ref()
            .map(|segmentation| EffectiveReaderTokenKey {
                document_id: self.document_id,
                page: self.page,
                segmentation: segmentation.clone(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderWordDetails {
    pub parts: Vec<crate::dictionary::DictionaryPart>,
    pub key: EffectiveReaderTokenKey,
    pub token: EffectiveReaderToken,
    pub encountered: u64,
    pub last_seen_unix_ms: Option<i64>,
    pub candidates: DictionaryCandidates,
}

/// Construction validates the entire snapshot before any page is projected.
/// Private fields prevent callers from replacing validated boundaries/metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderChapter {
    chapter: Arc<ChapterText>,
    segmentation: Arc<EffectiveChapterSegmentation>,
}

impl EffectiveReaderChapter {
    pub fn resolve(
        chapter: &ChapterText,
        original_id: AnalysisId,
        original: &ChapterAnalysis,
    ) -> Result<Self, InvalidAnalysis> {
        Self::resolve_shared(Arc::new(chapter.clone()), original_id, original)
    }

    pub fn resolve_shared(
        chapter: Arc<ChapterText>,
        original_id: AnalysisId,
        original: &ChapterAnalysis,
    ) -> Result<Self, InvalidAnalysis> {
        let segmentation = Arc::new(resolve_effective_segmentation(
            &chapter,
            original_id,
            original,
        )?);
        Ok(Self {
            chapter,
            segmentation,
        })
    }

    pub fn segmentation(&self) -> &Arc<EffectiveChapterSegmentation> {
        &self.segmentation
    }

    pub fn into_segmentation(self) -> Arc<EffectiveChapterSegmentation> {
        self.segmentation
    }

    /// Crossing tokens remain selectable on both pages with a complete surface,
    /// anchor and metadata. Only display_range is clipped to page-local UTF-8.
    pub fn tokens_on_page(
        &self,
        page: TextAnchor,
    ) -> Result<Vec<EffectiveReaderToken>, InvalidAnalysis> {
        self.project(page, false)
    }

    /// Completion owns each whole occurrence on the page containing its
    /// start. Repetitions remain separate; this function records no history.
    pub fn completion_tokens_on_page(
        &self,
        page: TextAnchor,
    ) -> Result<Vec<EffectiveReaderToken>, InvalidAnalysis> {
        self.project(page, true)
    }

    fn project(
        &self,
        page: TextAnchor,
        starts_only: bool,
    ) -> Result<Vec<EffectiveReaderToken>, InvalidAnalysis> {
        if page
            .slice(&self.chapter)
            .map_err(|_| InvalidAnalysis)?
            .is_empty()
        {
            return Err(InvalidAnalysis);
        }
        let segments = &self.segmentation.segments;
        let start = segments.partition_point(|segment| {
            if starts_only {
                segment.anchor.range.start < page.range.start
            } else {
                segment.anchor.range.end <= page.range.start
            }
        });
        segments[start..]
            .iter()
            .take_while(|segment| segment.anchor.range.start < page.range.end)
            .enumerate()
            .filter_map(|(index, segment)| {
                segment_eligibility(segment).map(|eligibility| {
                    Self::project_segment(page, start + index, segment, eligibility)
                })
            })
            .collect()
    }

    pub fn token_on_page(
        &self,
        page: TextAnchor,
        index: u64,
    ) -> Result<Option<EffectiveReaderToken>, InvalidAnalysis> {
        if page
            .slice(&self.chapter)
            .map_err(|_| InvalidAnalysis)?
            .is_empty()
        {
            return Err(InvalidAnalysis);
        }
        let index = usize::try_from(index).map_err(|_| InvalidAnalysis)?;
        self.segmentation
            .segments
            .get(index)
            .filter(|segment| {
                segment.anchor.range.start < page.range.end
                    && page.range.start < segment.anchor.range.end
            })
            .and_then(|segment| {
                segment_eligibility(segment)
                    .map(|eligibility| Self::project_segment(page, index, segment, eligibility))
            })
            .transpose()
    }

    fn project_segment(
        page: TextAnchor,
        index: usize,
        segment: &EffectiveSegment,
        eligibility: EffectiveTokenEligibility,
    ) -> Result<EffectiveReaderToken, InvalidAnalysis> {
        Ok(EffectiveReaderToken {
            segment_index: u64::try_from(index).map_err(|_| InvalidAnalysis)?,
            anchor: segment.anchor,
            display_range: TextRange {
                start: TextOffset::try_from(
                    i64::from(segment.anchor.range.start.max(page.range.start))
                        - i64::from(page.range.start),
                )
                .map_err(|_| InvalidAnalysis)?,
                end: TextOffset::try_from(
                    i64::from(segment.anchor.range.end.min(page.range.end))
                        - i64::from(page.range.start),
                )
                .map_err(|_| InvalidAnalysis)?,
            },
            surface: segment.surface.clone(),
            eligibility,
            metadata: segment.metadata.clone(),
            vocabulary: None,
        })
    }
}

fn segment_eligibility(segment: &EffectiveSegment) -> Option<EffectiveTokenEligibility> {
    segment
        .metadata
        .is_japanese
        .then_some(EffectiveTokenEligibility::AnalyzerJapanese)
}

/// Frozen effective completion. Only validated chapter projection can construct
/// occurrences; statuses are deliberately absent and remain live at commit.
/// The native session retains this value; full provenance stays outside UI payloads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectivePageCompletion {
    completion: PageCompletion,
    key: EffectiveReaderTokenKey,
    occurrences: Vec<EffectiveReaderToken>,
}

impl EffectivePageCompletion {
    pub fn completion(&self) -> PageCompletion {
        self.completion
    }

    pub fn key(&self) -> &EffectiveReaderTokenKey {
        &self.key
    }

    pub fn occurrences(&self) -> &[EffectiveReaderToken] {
        &self.occurrences
    }
}

impl EffectiveReaderChapter {
    pub fn freeze_completion(
        &self,
        completion: PageCompletion,
    ) -> Result<EffectivePageCompletion, InvalidAnalysis> {
        Ok(EffectivePageCompletion {
            key: EffectiveReaderTokenKey {
                document_id: completion.document_id,
                page: completion.page,
                segmentation: self.segmentation.clone(),
            },
            occurrences: self.completion_tokens_on_page(completion.page)?,
            completion,
        })
    }
}
