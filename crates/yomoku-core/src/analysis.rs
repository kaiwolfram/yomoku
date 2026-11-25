//! Validated analyzer output, without tokenization or I/O.

use serde::{Deserialize, Serialize};

use crate::{
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    vocabulary::Surface,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum SplitMode {
    A,
    B,
    #[default]
    C,
}

/// Import policy metadata is independent of the raw analyzer's A/B semantics.
pub const LEGACY_IMPORT_POLICY_SUFFIX: &str = ";yomoku-dictionary-refinement-v1";
pub const COMPOUND_IMPORT_POLICY_SUFFIX: &str = ";yomoku-dictionary-refinement-v2";
pub const IMPORT_POLICY_V3_SUFFIX: &str = ";yomoku-dictionary-refinement-v3";
pub const IMPORT_POLICY_V4_SUFFIX: &str = ";yomoku-dictionary-refinement-v4";
pub const IMPORT_POLICY_SUFFIX: &str = ";yomoku-dictionary-refinement-v5";
pub const WORD_DICTIONARY_PREFIX: &str = ";jmdict-jmnedict-sha256:";

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

/// Transient analyzer context aligned with Mode C tokens. Only the resulting
/// ChapterAnalysis is stored; dictionary components still use raw A/B tokens.
#[derive(Clone, Debug, Default)]
pub struct ImportTokenContext {
    pub starts_sentence: bool,
    pub standalone_grammar: bool,
    /// A preceding token cannot join across this boundary.
    pub compound_boundary_before: bool,
    pub inflected: bool,
    pub pending_ending: bool,
    /// A compound can include this stem only if it also includes the following verb.
    pub honorific_stem: bool,
    pub preserve_join: bool,
    /// Source prefix plus the analyzer lemma of each joined helper or causative.
    pub derived_verbs: Vec<Surface>,
    pub normalized_verb: Option<Surface>,
}

pub struct ImportAnalysis {
    pub analysis: ChapterAnalysis,
    pub context: Vec<ImportTokenContext>,
}

impl From<ChapterAnalysis> for ImportAnalysis {
    fn from(analysis: ChapterAnalysis) -> Self {
        Self {
            context: vec![ImportTokenContext::default(); analysis.tokens.len()],
            analysis,
        }
    }
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
