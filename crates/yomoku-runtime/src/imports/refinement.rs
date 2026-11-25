//! Dictionary-backed boundaries computed on the existing I/O worker.
use super::PasteAnalyzer;
use std::collections::HashMap;
use yomoku_core::{
    analysis::{
        AnalyzedToken, ChapterAnalysis, ImportAnalysis, ImportTokenContext, SplitMode,
        IMPORT_POLICY_SUFFIX, WORD_DICTIONARY_PREFIX,
    },
    document::{ChapterText, TextRange},
    storage::DocumentError,
    vocabulary::Surface,
};
use yomoku_storage::dictionary::{Dictionary, FormIndex, IndexedForm, IMPORT_DICTIONARY_SHA256};

pub(super) struct Refinement<'a> {
    dictionary: &'a Dictionary,
    index: FormIndex<'a>,
    forms: HashMap<String, IndexedForm>,
    word_forms: HashMap<Surface, bool>,
    readings: HashMap<String, Vec<String>>,
}

impl<'a> Refinement<'a> {
    pub(super) fn new(dictionary: &'a Dictionary) -> Result<Self, DocumentError> {
        Ok(Self {
            dictionary,
            index: dictionary
                .form_index()
                .map_err(|_| DocumentError::Unavailable)?,
            forms: HashMap::new(),
            word_forms: HashMap::new(),
            readings: HashMap::new(),
        })
    }

    fn indexed_form(&mut self, form: &str) -> Result<IndexedForm, DocumentError> {
        if let Some(indexed) = self.forms.get(form) {
            return Ok(*indexed);
        }
        let indexed = self
            .index
            .find(form)
            .map_err(|_| DocumentError::Unavailable)?;
        self.forms.insert(form.to_owned(), indexed);
        Ok(indexed)
    }

    fn contains(&mut self, form: &str) -> Result<bool, DocumentError> {
        Ok(self.indexed_form(form)? == IndexedForm::Exact)
    }

    fn refine_verb(
        &mut self,
        token: &mut AnalyzedToken,
        context: &ImportTokenContext,
    ) -> Result<(), DocumentError> {
        for candidate in context.derived_verbs.iter().rev() {
            let matches = if let Some(matches) = self.word_forms.get(candidate) {
                *matches
            } else {
                let matches = self
                    .dictionary
                    .contains_word(candidate.as_str())
                    .map_err(|_| DocumentError::Unavailable)?;
                self.word_forms.insert(candidate.clone(), matches);
                matches
            };
            if matches {
                token.dictionary_form = Some(candidate.clone());
                return Ok(());
            }
        }
        if let Some(normalized) = &context.normalized_verb {
            if !self.matches(token)? && self.contains(normalized.as_str())? {
                token.dictionary_form = Some(normalized.clone());
            }
        }
        Ok(())
    }

    /// Load applicable readings in dictionary order, with JMdict first.
    fn readings(&mut self, form: &str) -> Result<&[String], DocumentError> {
        if !self.readings.contains_key(form) {
            let candidates = self
                .dictionary
                .lookup(form, None)
                .map_err(|_| DocumentError::Unavailable)?;
            let mut readings = Vec::new();
            for word in candidates.words {
                for matched in word.forms.iter().flat_map(|form| &form.matches) {
                    readings.push(
                        word.entry.readings[matched.reading.reading_index]
                            .text
                            .clone(),
                    );
                }
            }
            for name in candidates.names {
                for matched in name.forms.iter().flat_map(|form| &form.matches) {
                    readings.push(name.entry.readings[matched.reading_index].text.clone());
                }
            }
            self.readings.insert(form.to_owned(), readings);
        }
        Ok(&self.readings[form])
    }

    fn longest_compound(
        &mut self,
        chapter: &ChapterText,
        tokens: &[AnalyzedToken],
        context: &[ImportTokenContext],
    ) -> Result<Option<(usize, AnalyzedToken)>, DocumentError> {
        let first = &tokens[0];
        let mut longest = None;
        let start = first.anchor.range.start;
        let mut end = first.anchor.range.end;
        if !first.is_japanese || context[0].standalone_grammar {
            return Ok(None);
        }
        for (index, (token, grammar)) in tokens.iter().zip(context).enumerate().skip(1) {
            if !token.is_japanese
                || (grammar.standalone_grammar && !grammar.compound_continuation)
                || grammar.starts_sentence
                || grammar.compound_boundary_before
                || end != token.anchor.range.start
            {
                break;
            }
            let prefix = TextRange { start, end }
                .slice(&chapter.text)
                .map_err(|_| DocumentError::InvalidDocument)?;
            if self.indexed_form(prefix)? == IndexedForm::Missing {
                break;
            }
            end = token.anchor.range.end;
            // Continue scanning so a complete honorific expression can still match.
            if grammar.honorific_stem {
                continue;
            }
            let surface = TextRange { start, end }
                .slice(&chapter.text)
                .map_err(|_| DocumentError::InvalidDocument)?;
            let indexed = self.indexed_form(surface)?;
            let exact = indexed == IndexedForm::Exact && !grammar.pending_ending;
            let mut reading = None;
            if exact && grammar.inflected {
                if let Some(analyzed) = compound_reading(&tokens[..=index]) {
                    reading = self
                        .readings(surface)?
                        .iter()
                        .find(|reading| reading.chars().map(hiragana).eq(analyzed.chars()))
                        .cloned();
                }
            }
            let exact = exact && (!grammar.inflected || reading.is_some());
            let mut base = None;
            // Keep the copular に literal within ように.
            if grammar.inflected && !grammar.compound_continuation {
                if let Some(lemma) = token
                    .dictionary_form
                    .as_ref()
                    .filter(|lemma| lemma.as_str() != token.surface)
                {
                    let candidate = format!("{prefix}{}", lemma.as_str());
                    if self.contains(&candidate)? {
                        base = Some(candidate);
                    }
                }
            }
            if exact || base.is_some() {
                longest = Some((index + 1, surface, end, exact, base, reading));
            }
        }
        let Some((consumed, surface, end, exact, base, reading)) = longest else {
            return Ok(None);
        };
        let mut compound = first.clone();
        compound.anchor.range.end = end;
        compound.surface = surface.to_owned();
        let base = base.as_deref().unwrap_or(surface);
        compound.dictionary_form = Surface::new(base).ok();
        compound.reading = if exact {
            match reading {
                Some(reading) => Some(reading),
                None => self.readings(surface)?.first().cloned(),
            }
        } else {
            compound_reading(&tokens[..consumed])
        };
        Ok(Some((consumed, compound)))
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
        let ImportAnalysis {
            analysis: mut original,
            context,
        } = analyzer.analyze_for_import(chapter)?;
        original
            .validate(chapter)
            .map_err(|_| DocumentError::InvalidDocument)?;
        if original.mode != SplitMode::C || original.tokens.len() != context.len() {
            return Err(DocumentError::InvalidDocument);
        }
        for (token, grammar) in original.tokens.iter_mut().zip(&context) {
            self.refine_verb(token, grammar)?;
        }
        let mut b = None;
        let mut a = None;
        let mut tokens = Vec::with_capacity(original.tokens.len());
        let mut index = 0;
        while let Some(token) = original.tokens.get(index) {
            if let Some((consumed, compound)) =
                self.longest_compound(chapter, &original.tokens[index..], &context[index..])?
            {
                tokens.push(compound);
                index += consumed;
                continue;
            }
            let preserve_join = context[index].preserve_join;
            index += 1;
            if !token.is_japanese || preserve_join || self.matches(token)? {
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

fn compound_reading(tokens: &[AnalyzedToken]) -> Option<String> {
    let mut reading = String::new();
    for token in tokens {
        reading.extend(token.reading.as_deref()?.chars().map(hiragana));
    }
    Some(reading)
}

fn hiragana(ch: char) -> char {
    match ch {
        'ァ'..='ヶ' | 'ヽ' | 'ヾ' => char::from_u32(u32::from(ch) - 0x60).unwrap_or(ch),
        _ => ch,
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

    fn pinned_dictionary() -> Dictionary {
        let path = std::env::var_os("YOMOKU_WORD_DICTIONARY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/dictionaries/index-v1-20261025/dictionary.sqlite3")
            });
        Dictionary::open(path).unwrap()
    }

    fn chapter(text: &str) -> ChapterText {
        ChapterText {
            source_id: 1.try_into().unwrap(),
            chapter_id: 1.try_into().unwrap(),
            text: text.into(),
        }
    }

    fn tokens(
        chapter: &ChapterText,
        parts: impl IntoIterator<Item = String>,
    ) -> Vec<AnalyzedToken> {
        let mut start = 0;
        parts
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
            .collect()
    }

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
            Ok(ChapterAnalysis {
                profile: profile.clone(),
                mode,
                tokens: tokens(chapter, parts),
            })
        }
    }

    #[test]
    #[ignore = "requires bundled word dictionary; run tools/test-sudachi.sh"]
    fn real_membership_keeps_larger_b_parts_refines_only_missing_parts_and_rejects_incomplete_coverage(
    ) {
        let dictionary = pinned_dictionary();
        for (text, expected) in [
            ("東京大学制服姿", vec!["東京大学", "制服", "姿"]),
            ("東京大学制服𰻞", vec!["東京大学制服𰻞"]),
        ] {
            let chapter = chapter(text);
            let result = Refinement::new(&dictionary)
                .unwrap()
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

    #[test]
    #[ignore = "requires bundled word dictionary; run tools/test-sudachi.sh"]
    fn real_compound_search_prefers_longest_then_leftmost_and_respects_grammar() {
        let dictionary = pinned_dictionary();
        let mut refinement = Refinement::new(&dictionary).unwrap();
        let context: [_; 3] = std::array::from_fn(|_| ImportTokenContext::default());
        // These token boundaries expose both the short and long forms, as well
        // as a match starting inside the earlier 日本語学校 match.
        for (parts, consumed, expected) in [
            (["その", "日", "暮らし"], 3, "その日暮らし"),
            (["日本語", "学校", "教育"], 2, "日本語学校"),
        ] {
            let chapter = chapter(&parts.concat());
            let tokens = tokens(&chapter, parts.map(str::to_owned));
            let (count, token) = refinement
                .longest_compound(&chapter, &tokens, &context)
                .unwrap()
                .unwrap();
            assert_eq!((count, token.surface.as_str()), (consumed, expected));
            assert_eq!(token.anchor.slice(&chapter).unwrap(), expected);
        }

        let name = chapter("神宮前");
        let name_tokens = tokens(&name, ["神宮", "前"].map(str::to_owned));
        let (_, token) = refinement
            .longest_compound(&name, &name_tokens, &context[..2])
            .unwrap()
            .unwrap();
        assert_eq!(token.reading.as_deref(), Some("じんぐうまえ"));

        let phrase = chapter("その日");
        let phrase_tokens = tokens(&phrase, ["その", "日"].map(str::to_owned));
        for blocked in [
            ImportTokenContext {
                starts_sentence: true,
                ..Default::default()
            },
            ImportTokenContext {
                standalone_grammar: true,
                ..Default::default()
            },
            ImportTokenContext {
                pending_ending: true,
                ..Default::default()
            },
        ] {
            assert!(refinement
                .longest_compound(&phrase, &phrase_tokens, &[context[0].clone(), blocked])
                .unwrap()
                .is_none());
        }
    }

    #[test]
    #[ignore = "requires bundled word dictionary; run tools/test-sudachi.sh"]
    fn inflected_exact_match_uses_the_agreeing_reading() {
        let dictionary = pinned_dictionary();
        let mut refinement = Refinement::new(&dictionary).unwrap();
        let chapter = chapter("端っこ");
        let mut tokens = tokens(&chapter, ["端", "っこ"].map(str::to_owned));
        let context = [
            ImportTokenContext::default(),
            ImportTokenContext {
                inflected: true,
                ..Default::default()
            },
        ];
        tokens[0].reading = Some("ハジ".into());
        assert!(refinement
            .longest_compound(&chapter, &tokens, &context)
            .unwrap()
            .is_none());
        tokens[1].reading = Some("ッコ".into());
        let (_, compound) = refinement
            .longest_compound(&chapter, &tokens, &context)
            .unwrap()
            .unwrap();
        assert_eq!(compound.reading.as_deref(), Some("はじっこ"));
    }
}
