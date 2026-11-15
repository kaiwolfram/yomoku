//! Dictionary values and exact-form restriction rules. No I/O or analyzer-reading
//! filter: a written surface may have several valid readings. User fallbacks stay
//! separate from immutable built-in payloads.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct Spelling {
    pub text: String,
    #[serde(default)]
    pub ke_inf: Vec<String>,
    #[serde(default)]
    pub ke_pri: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub text: String,
    #[serde(default)]
    pub no_kanji: bool,
    #[serde(default)]
    pub re_restr: Vec<String>,
    #[serde(default)]
    pub re_inf: Vec<String>,
    #[serde(default)]
    pub re_pri: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct Translation {
    pub text: String,
    pub lang: String,
    pub g_type: Option<String>,
    pub g_gend: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct LoanSource {
    pub text: String,
    pub lang: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub wasei: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct Sense {
    #[serde(default)]
    pub stagk: Vec<String>,
    #[serde(default)]
    pub stagr: Vec<String>,
    #[serde(default)]
    pub pos: Vec<String>,
    #[serde(default)]
    pub field: Vec<String>,
    #[serde(default)]
    pub misc: Vec<String>,
    #[serde(default)]
    pub s_inf: Vec<String>,
    #[serde(default)]
    pub dial: Vec<String>,
    #[serde(default)]
    pub xref: Vec<String>,
    #[serde(default)]
    pub ant: Vec<String>,
    #[serde(default)]
    pub loan_sources: Vec<LoanSource>,
    pub glosses: Vec<Translation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct NameGroup {
    #[serde(default)]
    pub name_type: Vec<String>,
    #[serde(default)]
    pub xref: Vec<String>,
    pub translations: Vec<Translation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct WordEntry {
    pub kanji: Vec<Spelling>,
    pub readings: Vec<Reading>,
    pub senses: Vec<Sense>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[serde(deny_unknown_fields)]
pub struct NameEntry {
    pub kanji: Vec<Spelling>,
    pub readings: Vec<Reading>,
    pub names: Vec<NameGroup>,
}

/// Indices refer to the original ordered payload, which remains intact.
/// A kana query can select multiple spelling/reading pairs. `None` denotes a
/// kana-only reading, including re_nokanji; it cannot satisfy a stagk restriction.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct ReadingMatch {
    pub spelling_index: Option<usize>,
    pub reading_index: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct WordMatch {
    pub reading: ReadingMatch,
    pub sense_indices: Vec<usize>,
}

/// Select every spelling/reading pair licensed by an exact written OR kana form.
/// Never normalize kana, fold Unicode, or select by the analyzer's one reading.
pub fn matching_readings(
    kanji: &[Spelling],
    readings: &[Reading],
    form: &str,
) -> Vec<ReadingMatch> {
    let mut matches = Vec::new();
    for (reading_index, reading) in readings.iter().enumerate() {
        let reading_matches = reading.text == form;
        if reading.no_kanji || kanji.is_empty() {
            if reading_matches {
                matches.push(ReadingMatch {
                    spelling_index: None,
                    reading_index,
                });
            }
            continue;
        }
        for (spelling_index, spelling) in kanji.iter().enumerate() {
            if (reading_matches || spelling.text == form)
                && (reading.re_restr.is_empty() || reading.re_restr.contains(&spelling.text))
            {
                matches.push(ReadingMatch {
                    spelling_index: Some(spelling_index),
                    reading_index,
                });
            }
        }
    }
    matches
}

impl WordEntry {
    /// Keep senses in source order, applying BOTH spelling and reading limits.
    /// Empty-sense pairs are omitted; callers display only these selected senses,
    /// never the entire unfiltered candidate payload.
    pub fn matches(&self, form: &str) -> Vec<WordMatch> {
        matching_readings(&self.kanji, &self.readings, form)
            .into_iter()
            .filter_map(|reading| {
                let spelling = reading.spelling_index.map(|i| &self.kanji[i].text);
                let text = &self.readings[reading.reading_index].text;
                let sense_indices: Vec<_> = self
                    .senses
                    .iter()
                    .enumerate()
                    .filter(|(_, sense)| {
                        (sense.stagk.is_empty()
                            || spelling.is_some_and(|s| sense.stagk.contains(s)))
                            && (sense.stagr.is_empty() || sense.stagr.contains(text))
                    })
                    .map(|(i, _)| i)
                    .collect();
                (!sense_indices.is_empty()).then_some(WordMatch {
                    reading,
                    sense_indices,
                })
            })
            .collect()
    }
}

/// Each candidate retains all upstream fields and applicable per-form indices.
/// Concrete word/name types keep the generated bridge schema unambiguous.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct WordCandidate {
    /// Positive source-scoped ID; word and name IDs may overlap.
    pub entry_id: i64,
    pub entry: WordEntry,
    pub forms: Vec<WordFormMatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct WordFormMatch {
    pub form: String,
    pub matches: Vec<WordMatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct NameCandidate {
    /// Positive source-scoped ID; word and name IDs may overlap.
    pub entry_id: i64,
    pub entry: NameEntry,
    pub forms: Vec<NameFormMatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct NameFormMatch {
    pub form: String,
    pub matches: Vec<ReadingMatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct DictionaryCandidates {
    pub surface: String,
    pub dictionary_form: Option<String>,
    pub words: Vec<WordCandidate>,
    pub names: Vec<NameCandidate>,
}

/// Validated analyzer provenance for an entry form, including inflected components.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct DictionarySource {
    pub form: String,
    pub anchor: crate::document::TextAnchor,
}

/// Order Japanese components by length then source position, keeping the first
/// occurrence of each exact surface/base form and its original highlight anchor.
/// Callers supply the supported analyses and any selection-range filtering.
pub fn component_sources(mut tokens: Vec<crate::analysis::AnalyzedToken>) -> Vec<DictionarySource> {
    tokens.retain(|token| token.is_japanese);
    tokens.sort_by_key(|token| {
        (
            std::cmp::Reverse(token.surface.chars().count()),
            token.anchor.range.start,
        )
    });
    let mut sources: Vec<DictionarySource> = Vec::new();
    for token in tokens {
        for form in std::iter::once(token.surface)
            .chain(token.dictionary_form.map(|form| form.as_str().to_owned()))
        {
            if !sources.iter().any(|source| source.form == form) {
                sources.push(DictionarySource {
                    form,
                    anchor: token.anchor,
                });
            }
        }
    }
    sources
}
