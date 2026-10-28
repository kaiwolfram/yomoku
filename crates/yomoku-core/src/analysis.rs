//! Validated analyzer output and reversible boundary edits, without tokenization or I/O.

use serde::{Deserialize, Serialize};

use crate::{
    document::{ChapterText, OverrideId, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum SplitMode {
    A,
    #[default]
    B,
    C,
}

/// Exact engine revision and dictionary artifact identity, not a moving "latest".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnalysisProfile {
    pub engine_revision: String,
    pub dictionary_revision: String,
}

/// Surface is copied from the original source, never normalized by the analyzer.
/// Classification/lemma/reading are supplied by the analyzer, not guessed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalyzedToken {
    pub anchor: TextAnchor,
    pub surface: String,
    pub is_japanese: bool,
    pub reading: Option<String>,
    pub dictionary_form: Option<Surface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterAnalysis {
    pub profile: AnalysisProfile,
    pub mode: SplitMode,
    pub tokens: Vec<AnalyzedToken>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidAnalysis;

impl std::fmt::Display for InvalidAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid analysis or segmentation anchor")
    }
}

impl std::error::Error for InvalidAnalysis {}

impl ChapterAnalysis {
    /// Assign a whole Japanese occurrence to the page containing its start.
    /// A token crossing a logical page boundary is counted exactly once, with
    /// its original full anchor. Page derivation is validated by the caller.
    pub fn japanese_tokens_on_page(
        &self,
        chapter: &ChapterText,
        page: TextAnchor,
    ) -> Result<Vec<(usize, &AnalyzedToken)>, InvalidAnalysis> {
        self.validate(chapter)?;
        page.slice(chapter).map_err(|_| InvalidAnalysis)?;
        Ok(self
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, token)| {
                token.is_japanese
                    && token.anchor.range.start >= page.range.start
                    && token.anchor.range.start < page.range.end
            })
            .collect())
    }

    pub fn validate(&self, chapter: &ChapterText) -> Result<(), InvalidAnalysis> {
        if self.profile.engine_revision.trim().is_empty()
            || self.profile.dictionary_revision.trim().is_empty()
        {
            return Err(InvalidAnalysis);
        }
        let mut previous_end = TextOffset::START;
        for token in &self.tokens {
            let source = token.anchor.slice(chapter).map_err(|_| InvalidAnalysis)?;
            if source.is_empty()
                || source != token.surface
                || token.anchor.range.start < previous_end
                || token.reading.as_ref().is_some_and(|s| s.trim().is_empty())
                || (token.is_japanese && Surface::new(source).is_err())
            {
                return Err(InvalidAnalysis);
            }
            let gap = TextRange {
                start: previous_end,
                end: token.anchor.range.start,
            }
            .slice(&chapter.text)
            .map_err(|_| InvalidAnalysis)?;
            if !gap.chars().all(char::is_whitespace) {
                return Err(InvalidAnalysis);
            }
            previous_end = token.anchor.range.end;
        }
        let tail = TextRange {
            start: previous_end,
            end: TextOffset::at(&chapter.text, chapter.text.len()).map_err(|_| InvalidAnalysis)?,
        }
        .slice(&chapter.text)
        .map_err(|_| InvalidAnalysis)?;
        if !tail.chars().all(char::is_whitespace) {
            return Err(InvalidAnalysis);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum OverrideScope {
    #[default]
    AllOccurrences,
    ThisOccurrence,
}

/// A selected exact source span partitioned into one (merge) or more (split)
/// surfaces. Retaining the analyzer snapshot makes removing this edit reversible.
/// This carries no user-supplied lemma/reading and never rewrites past encounters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentationOverride {
    pub scope: OverrideScope,
    pub anchor: TextAnchor,
    pub surface: Surface,
    pub parts: Vec<Surface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredOverride {
    pub id: OverrideId,
    pub edit: SegmentationOverride,
    pub active: bool,
}

impl SegmentationOverride {
    /// Also usable after source text deletion: validate the retained exact span
    /// length and its partition. Full source ownership is checked by repositories.
    pub fn validate_shape(&self) -> Result<(), InvalidAnalysis> {
        let length = i64::from(self.anchor.range.end)
            .checked_sub(i64::from(self.anchor.range.start))
            .ok_or(InvalidAnalysis)?;
        if length <= 0
            || usize::try_from(length).ok() != Some(self.surface.as_str().len())
            || self.parts.is_empty()
        {
            return Err(InvalidAnalysis);
        }
        let mut remaining = self.surface.as_str();
        for part in &self.parts {
            remaining = remaining
                .strip_prefix(part.as_str())
                .ok_or(InvalidAnalysis)?;
        }
        if !remaining.is_empty() {
            return Err(InvalidAnalysis);
        }
        Ok(())
    }

    /// Return replacement anchors for this selected occurrence only. Applying an
    /// AllOccurrences rule to other matches is a separate resolution operation.
    pub fn replacement_anchors(
        &self,
        chapter: &ChapterText,
    ) -> Result<Vec<TextAnchor>, InvalidAnalysis> {
        self.validate_shape()?;
        if self.anchor.slice(chapter).map_err(|_| InvalidAnalysis)? != self.surface.as_str() {
            return Err(InvalidAnalysis);
        }
        let mut start = self
            .anchor
            .range
            .start
            .byte_index(&chapter.text)
            .map_err(|_| InvalidAnalysis)?;
        self.parts
            .iter()
            .map(|part| {
                let end = start
                    .checked_add(part.as_str().len())
                    .ok_or(InvalidAnalysis)?;
                let range = TextRange {
                    start: TextOffset::at(&chapter.text, start).map_err(|_| InvalidAnalysis)?,
                    end: TextOffset::at(&chapter.text, end).map_err(|_| InvalidAnalysis)?,
                };
                start = end;
                Ok(TextAnchor {
                    range,
                    ..self.anchor
                })
            })
            .collect()
    }
}
