//! Dictionary-backed boundaries computed on the existing I/O worker.
use super::PasteAnalyzer;
use std::collections::HashMap;
use yomoku_core::{
    analysis::{
        AnalyzedToken, ChapterAnalysis, SplitMode, IMPORT_POLICY_SUFFIX, WORD_DICTIONARY_PREFIX,
    },
    document::ChapterText,
    storage::DocumentError,
};
use yomoku_storage::dictionary::{Dictionary, IMPORT_DICTIONARY_SHA256};

pub(super) struct Refinement<'a> {
    dictionary: &'a Dictionary,
    membership: HashMap<String, bool>,
}

impl<'a> Refinement<'a> {
    pub(super) fn new(dictionary: &'a Dictionary) -> Self {
        Self {
            dictionary,
            membership: HashMap::new(),
        }
    }

    fn contains(&mut self, form: &str) -> Result<bool, DocumentError> {
        if let Some(found) = self.membership.get(form) {
            return Ok(*found);
        }
        let found = self
            .dictionary
            .contains(form)
            .map_err(|_| DocumentError::Unavailable)?;
        self.membership.insert(form.to_owned(), found);
        Ok(found)
    }

    fn matches(&mut self, token: &AnalyzedToken) -> Result<bool, DocumentError> {
        if self.contains(&token.surface)? {
            return Ok(true);
        }
        match token.dictionary_form.as_ref() {
            Some(base) => self.contains(base.as_str()),
            None => Ok(false),
        }
    }

    /// Require contiguous finer tokens covering the exact parent span. Unmatched
    /// pieces, source gaps and crossing components keep the original token.
    fn components(
        &mut self,
        parent: &AnalyzedToken,
        finer: &[AnalyzedToken],
        finest: Option<&[AnalyzedToken]>,
    ) -> Result<Option<Vec<AnalyzedToken>>, DocumentError> {
        let mut position = parent.anchor.range.start;
        let mut result = Vec::new();
        let start =
            finer.partition_point(|token| token.anchor.range.start < parent.anchor.range.start);
        for token in finer[start..]
            .iter()
            .take_while(|token| token.anchor.range.start < parent.anchor.range.end)
        {
            if token.anchor.range.start != position
                || token.anchor.range.end > parent.anchor.range.end
            {
                return Ok(None);
            }
            if self.matches(token)? {
                result.push(token.clone());
            } else if let Some(finest) = finest {
                let Some(parts) = self.components(token, finest, None)? else {
                    return Ok(None);
                };
                result.extend(parts);
            } else {
                return Ok(None);
            }
            position = token.anchor.range.end;
        }
        Ok((position == parent.anchor.range.end && !result.is_empty()).then_some(result))
    }

    pub(super) fn analyze(
        &mut self,
        analyzer: &mut dyn PasteAnalyzer,
        chapter: &ChapterText,
    ) -> Result<ChapterAnalysis, DocumentError> {
        let mut original = analyzer.analyze(chapter)?;
        original
            .validate(chapter)
            .map_err(|_| DocumentError::InvalidDocument)?;
        if original.mode != SplitMode::C {
            return Err(DocumentError::InvalidDocument);
        }
        let mut b = None;
        let mut a = None;
        let mut tokens = Vec::with_capacity(original.tokens.len());
        for token in &original.tokens {
            if !token.is_japanese || self.matches(token)? {
                tokens.push(token.clone());
                continue;
            }
            if b.is_none() {
                b = Some(components(analyzer, chapter, SplitMode::B, &original)?);
            }
            let b = &b.as_ref().expect("loaded B").tokens;
            let replacement = match self.components(token, b, None)? {
                Some(parts) => Some(parts),
                None => {
                    if a.is_none() {
                        a = Some(components(analyzer, chapter, SplitMode::A, &original)?);
                    }
                    self.components(token, b, Some(&a.as_ref().expect("loaded A").tokens))?
                }
            };
            match replacement {
                Some(parts) => tokens.extend(parts),
                None => tokens.push(token.clone()),
            }
        }
        original.tokens = tokens;
        original
            .profile
            .engine_revision
            .push_str(IMPORT_POLICY_SUFFIX);
        original
            .profile
            .dictionary_revision
            .push_str(WORD_DICTIONARY_PREFIX);
        original
            .profile
            .dictionary_revision
            .push_str(IMPORT_DICTIONARY_SHA256);
        original
            .validate(chapter)
            .map_err(|_| DocumentError::InvalidDocument)?;
        Ok(original)
    }
}

fn components(
    analyzer: &mut dyn PasteAnalyzer,
    chapter: &ChapterText,
    mode: SplitMode,
    original: &ChapterAnalysis,
) -> Result<ChapterAnalysis, DocumentError> {
    let finer = analyzer.analyze_mode(chapter, mode, &original.profile)?;
    finer
        .validate(chapter)
        .map_err(|_| DocumentError::InvalidDocument)?;
    if finer.mode != mode || finer.profile != original.profile {
        return Err(DocumentError::InvalidDocument);
    }
    Ok(finer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use yomoku_core::{
        analysis::AnalysisProfile,
        document::{TextAnchor, TextOffset, TextRange},
        vocabulary::Surface,
    };

    struct FixtureAnalyzer;
    impl PasteAnalyzer for FixtureAnalyzer {
        fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
            self.analyze_mode(
                chapter,
                SplitMode::C,
                &AnalysisProfile {
                    engine_revision: "fixture".into(),
                    dictionary_revision: "fixture".into(),
                },
            )
        }
        fn analyze_mode(
            &mut self,
            chapter: &ChapterText,
            mode: SplitMode,
            profile: &AnalysisProfile,
        ) -> Result<ChapterAnalysis, DocumentError> {
            let last = if chapter.text.ends_with('姿') {
                "姿"
            } else {
                "𰻞"
            };
            let parts = match mode {
                SplitMode::C => vec![chapter.text.clone()],
                SplitMode::B => vec!["東京大学".into(), format!("制服{last}")],
                SplitMode::A => vec!["東京".into(), "大学".into(), "制服".into(), last.into()],
            };
            let mut start = 0;
            let tokens = parts
                .into_iter()
                .map(|surface| {
                    let end = start + surface.len();
                    let token = AnalyzedToken {
                        anchor: TextAnchor {
                            source_id: chapter.source_id,
                            chapter_id: chapter.chapter_id,
                            range: TextRange {
                                start: TextOffset::at(&chapter.text, start).unwrap(),
                                end: TextOffset::at(&chapter.text, end).unwrap(),
                            },
                        },
                        dictionary_form: Some(Surface::new(&surface).unwrap()),
                        surface,
                        is_japanese: true,
                        reading: None,
                    };
                    start = end;
                    token
                })
                .collect();
            Ok(ChapterAnalysis {
                profile: profile.clone(),
                mode,
                tokens,
            })
        }
    }

    #[test]
    #[ignore = "requires bundled word dictionary; run tools/test-sudachi.sh"]
    fn real_membership_keeps_larger_b_parts_refines_only_missing_parts_and_rejects_incomplete_coverage(
    ) {
        let path = std::env::var_os("YOMOKU_WORD_DICTIONARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/dictionaries/index-v1-20261025/dictionary.sqlite3")
            });
        let dictionary = Dictionary::open(path).unwrap();
        for (text, expected) in [
            ("東京大学制服姿", vec!["東京大学", "制服", "姿"]),
            ("東京大学制服𰻞", vec!["東京大学制服𰻞"]),
        ] {
            let chapter = ChapterText {
                source_id: 1.try_into().unwrap(),
                chapter_id: 1.try_into().unwrap(),
                text: text.into(),
            };
            let result = Refinement::new(&dictionary)
                .analyze(&mut FixtureAnalyzer, &chapter)
                .unwrap();
            assert_eq!(
                result
                    .tokens
                    .iter()
                    .map(|t| t.surface.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            result.validate(&chapter).unwrap();
        }
    }
}
