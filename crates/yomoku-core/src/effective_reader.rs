//! Read-only reader projections over validated effective segmentation.

use serde::{Deserialize, Serialize};

use crate::{
    analysis::{ChapterAnalysis, InvalidAnalysis, SplitMode, StoredOverride},
    completion::PageCompletion,
    dictionary::DictionaryCandidates,
    document::{
        AnalysisId, ChapterText, DocumentId, InvalidTextValue, TextAnchor, TextOffset, TextRange,
    },
    reader::{ReaderCustomEntry, ReaderVocabulary},
    reader_ruby::{source_preferred_ruby, ReaderChapterRuby, ReaderRuby},
    segmentation::{
        resolve_effective_segmentation, EffectiveChapterSegmentation, EffectiveSegment,
        SegmentMetadata,
    },
};

/// Reading eligibility is distinct from analyzer metadata. A manual partition
/// inherits the editable Japanese B span; it does not acquire an analyzer lemma,
/// reading or classification. Exact analyzer output, when available, takes priority.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum EffectiveTokenEligibility {
    AnalyzerJapanese,
    ManualJapaneseSpan,
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

/// A separate repository projection until live selection/completion commands can
/// freeze and validate effective identities together. None segmentation means
/// missing original B; it never requests analysis or creates vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveReaderTokenSnapshot {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: Option<EffectiveChapterSegmentation>,
    pub tokens: Vec<EffectiveReaderToken>,
    pub original_boundaries: Vec<OriginalBoundaryToken>,
}

/// Original Japanese B spans for editing, independent of the effective partition.
/// Full edge tokens are retained; only their display range is clipped to the page.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct OriginalBoundaryToken {
    pub anchor: TextAnchor,
    pub display_range: TextRange,
    pub surface: String,
}

pub fn original_boundaries_on_page(
    chapter: &ChapterText,
    original: &ChapterAnalysis,
    page: TextAnchor,
) -> Result<Vec<OriginalBoundaryToken>, InvalidAnalysis> {
    original.validate(chapter)?;
    if original.mode != SplitMode::B || page.slice(chapter).map_err(|_| InvalidAnalysis)?.is_empty()
    {
        return Err(InvalidAnalysis);
    }
    original
        .tokens
        .iter()
        .filter(|token| {
            token.is_japanese
                && token.anchor.range.start < page.range.end
                && token.anchor.range.end > page.range.start
        })
        .map(|token| {
            Ok(OriginalBoundaryToken {
                anchor: token.anchor,
                display_range: TextRange {
                    start: (i64::from(token.anchor.range.start.max(page.range.start))
                        - i64::from(page.range.start))
                    .try_into()
                    .map_err(|_| InvalidAnalysis)?,
                    end: (i64::from(token.anchor.range.end.min(page.range.end))
                        - i64::from(page.range.start))
                    .try_into()
                    .map_err(|_| InvalidAnalysis)?,
                },
                surface: token.surface.clone(),
            })
        })
        .collect()
}

/// Frozen page and complete resolved provenance, deliberately separate from an
/// original analyzer identity. Live vocabulary/status changes do not invalidate
/// it; changed boundaries or metadata do. Not yet part of the public UI bridge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectiveReaderTokenKey {
    pub document_id: DocumentId,
    pub page: TextAnchor,
    pub segmentation: EffectiveChapterSegmentation,
}

impl EffectiveReaderTokenSnapshot {
    /// Missing original B cannot produce a selectable effective identity.
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
    /// User data remains separate from unresolved analyzer metadata.
    pub custom_entry: Option<ReaderCustomEntry>,
}

/// Construction validates entire snapshots/rules before any page is projected.
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
        alternates: &[(AnalysisId, ChapterAnalysis)],
        rules: &[StoredOverride],
    ) -> Result<Self, InvalidAnalysis> {
        Ok(Self {
            segmentation: resolve_effective_segmentation(
                chapter,
                original_id,
                original,
                alternates,
                rules,
            )?,
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

    /// A future completion owns each whole occurrence on the page containing its
    /// start. Repetitions remain separate; this function records no history.
    pub fn completion_tokens_on_page(
        &self,
        page: TextAnchor,
    ) -> Result<Vec<EffectiveReaderToken>, InvalidAnalysis> {
        self.project(page, true)
    }

    /// Source ruby wins over exact-span effective analyzer readings. Unresolved
    /// manual parts have no reading fallback; never reuse their old B readings.
    /// The validated chapter owns metadata, independently of live vocabulary.
    /// Crossing bases retain the complete reading on every intersecting page.
    pub fn furigana_on_page(
        &self,
        page: TextAnchor,
        source: Option<&ReaderChapterRuby>,
    ) -> Result<Vec<ReaderRuby>, InvalidTextValue> {
        let tokens = self.tokens_on_page(page).map_err(|_| InvalidTextValue)?;
        let source = source
            .map(|source| source.on_page(self.chapter, page.range))
            .transpose()?
            .unwrap_or_default();
        Ok(source_preferred_ruby(
            &source,
            tokens.into_iter().filter_map(|token| {
                token.metadata?.reading.map(|reading| ReaderRuby {
                    range: token.display_range,
                    reading,
                })
            }),
        ))
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
        None if segment.boundary.override_id.is_some() => {
            Some(EffectiveTokenEligibility::ManualJapaneseSpan)
        }
        None => None,
    }
}

/// Frozen effective completion. Only validated chapter projection can construct
/// occurrences; statuses are deliberately absent and remain live at commit.
/// This prerequisite is not yet a public UI/FFI payload.
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
