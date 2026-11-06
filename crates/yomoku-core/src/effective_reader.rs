//! Read-only reader projections over validated effective segmentation.

use serde::{Deserialize, Serialize};

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
    /// This is never an original analyzer token index.
    pub segment_index: u64,
    pub anchor: TextAnchor,
    pub display_range: TextRange,
    pub surface: String,
    pub eligibility: EffectiveTokenEligibility,
    pub metadata: Option<SegmentMetadata>,
    pub vocabulary: Option<ReaderVocabulary>,
}

/// Page lookup with resolved provenance. Missing segmentation means absent
/// original C; lookup never requests analysis or creates vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderTokenSnapshot {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: Option<EffectiveChapterSegmentation>,
    pub tokens: Vec<EffectiveReaderToken>,
}

/// Frozen page and complete resolved provenance, deliberately separate from an
/// original analyzer identity. Live vocabulary/status changes do not invalidate
/// it; changed boundaries or metadata do. Full keys remain native.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveReaderTokenKey {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: EffectiveChapterSegmentation,
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
    pub key: EffectiveReaderTokenKey,
    pub token: EffectiveReaderToken,
    pub encountered: u64,
    pub candidates: DictionaryCandidates,

    pub dictionary_sources: Vec<crate::dictionary::DictionarySource>,
}

/// Construction validates the entire snapshot before any page is projected.
/// Private fields prevent callers from replacing validated boundaries/metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderChapter<'a> {
    chapter: &'a ChapterText,
    segmentation: EffectiveChapterSegmentation,
}

impl<'a> EffectiveReaderChapter<'a> {
    pub fn resolve(
        chapter: &'a ChapterText,
        original_id: AnalysisId,
        original: &ChapterAnalysis,
    ) -> Result<Self, InvalidAnalysis> {
        Ok(Self {
            segmentation: resolve_effective_segmentation(chapter, original_id, original)?,
            chapter,
        })
    }

    pub fn segmentation(&self) -> &EffectiveChapterSegmentation {
        &self.segmentation
    }

    pub fn into_segmentation(self) -> EffectiveChapterSegmentation {
        self.segmentation
    }

    /// Borrow whole-chapter reading surfaces without allocating page tokens or
    /// cloning metadata. Library membership uses the same eligibility as reading
    /// and completion; callers deduplicate exact spellings across chapters.
    pub fn eligible_surfaces(&self) -> impl Iterator<Item = &str> {
        self.segmentation.segments.iter().filter_map(|segment| {
            segment_eligibility(segment).map(|_| segment.boundary.surface.as_str())
        })
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
            .slice(self.chapter)
            .map_err(|_| InvalidAnalysis)?
            .is_empty()
        {
            return Err(InvalidAnalysis);
        }
        let mut tokens = Vec::new();
        for (index, segment) in self.segmentation.segments.iter().enumerate() {
            let boundary = &segment.boundary;
            let Some(eligibility) = segment_eligibility(segment) else {
                continue;
            };
            if boundary.anchor.range.start >= page.range.end
                || boundary.anchor.range.end <= page.range.start
                || (starts_only && boundary.anchor.range.start < page.range.start)
            {
                continue;
            }
            tokens.push(EffectiveReaderToken {
                segment_index: u64::try_from(index).map_err(|_| InvalidAnalysis)?,
                anchor: boundary.anchor,
                display_range: TextRange {
                    start: TextOffset::try_from(
                        i64::from(boundary.anchor.range.start.max(page.range.start))
                            - i64::from(page.range.start),
                    )
                    .map_err(|_| InvalidAnalysis)?,
                    end: TextOffset::try_from(
                        i64::from(boundary.anchor.range.end.min(page.range.end))
                            - i64::from(page.range.start),
                    )
                    .map_err(|_| InvalidAnalysis)?,
                },
                surface: boundary.surface.clone(),
                eligibility,
                metadata: segment.metadata.clone(),
                vocabulary: None,
            });
        }
        Ok(tokens)
    }
}

fn segment_eligibility(segment: &EffectiveSegment) -> Option<EffectiveTokenEligibility> {
    match &segment.metadata {
        Some(metadata) if metadata.is_japanese => Some(EffectiveTokenEligibility::AnalyzerJapanese),
        Some(_) => None,
        None => None,
    }
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

impl EffectiveReaderChapter<'_> {
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
