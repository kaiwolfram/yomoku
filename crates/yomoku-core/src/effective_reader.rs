//! Read-only reader projections over validated effective segmentation.

use crate::{
    analysis::{ChapterAnalysis, InvalidAnalysis, StoredOverride},
    document::{AnalysisId, ChapterText, DocumentId, TextAnchor, TextOffset, TextRange},
    reader::ReaderVocabulary,
    segmentation::{resolve_effective_segmentation, EffectiveChapterSegmentation, SegmentMetadata},
};

/// Reading eligibility is distinct from analyzer metadata. A manual partition
/// inherits the editable Japanese B span; it does not acquire an analyzer lemma,
/// reading or classification. Exact analyzer output, when available, takes priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectiveTokenEligibility {
    AnalyzerJapanese,
    ManualJapaneseSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
            let eligibility = match &segment.metadata {
                Some(metadata) if metadata.is_japanese => {
                    EffectiveTokenEligibility::AnalyzerJapanese
                }
                Some(_) => continue,
                None if boundary.override_id.is_some() => {
                    EffectiveTokenEligibility::ManualJapaneseSpan
                }
                None => continue,
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
