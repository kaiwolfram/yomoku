//! Production Mode C post-processing; A/B output stays raw for dictionary lookup.

use yomoku_core::{
    analysis::{AnalyzedToken, ImportTokenContext},
    vocabulary::Surface,
};

pub(super) struct Morpheme {
    pub token: AnalyzedToken,
    // Sudachi's six POS fields include conjugation type (4) and form (5).
    pub pos: Vec<String>,
    pub normalized_form: String,
}

impl Morpheme {
    fn pos(&self, index: usize) -> &str {
        self.pos.get(index).map_or("*", String::as_str)
    }

    fn conjugates(&self) -> bool {
        self.pos(4) != "*" && self.pos(5) != "*"
    }

    fn head(&self) -> bool {
        self.token.is_japanese
            && (matches!(self.pos(0), "動詞" | "形容詞")
                || (self.pos(0) == "助動詞"
                    && matches!(self.normalized_form.as_str(), "だ" | "です")))
            && self.conjugates()
    }

    fn continuative(&self) -> bool {
        self.conjugates() && self.pos(5).starts_with("連用形")
    }

    fn adjectival(&self) -> bool {
        self.pos(0) == "形容詞" || (self.pos(0) == "接尾辞" && self.pos(1) == "形容詞的")
    }

    fn ending_after(&self, previous: &Self) -> bool {
        self.token.is_japanese
            && self.conjugates()
            && previous.conjugates()
            && (self.pos(0) == "助動詞"
                || (self.pos(0) == "形容詞"
                    && self.pos(1) == "非自立可能"
                    && self.normalized_form == "無い"
                    && previous.adjectival()
                    && previous.continuative()))
    }

    fn connector(&self) -> bool {
        self.pos(0) == "助詞"
            && self.pos(1) == "接続助詞"
            && matches!(self.token.surface.as_str(), "て" | "で")
    }

    fn helper(&self) -> bool {
        self.pos(0) == "動詞"
            && self.pos(1) == "非自立可能"
            && self.conjugates()
            // Normalized dictionary lemmas handle kanji and kana spellings;
            // source surfaces and the head's dictionary form remain untouched.
            && matches!(
                self.normalized_form.as_str(),
                "居る" | "有る" | "おく" | "置く" | "仕舞う" | "見る" | "行く" | "来る"
                    | "呉れる" | "貰う" | "上げる"
            )
    }

    fn adjacent(&self, next: &Self) -> bool {
        self.token.anchor.range.end == next.token.anchor.range.start
    }

    fn counter_after(&self, previous: &Self) -> bool {
        previous.pos(0) == "名詞"
            && previous.pos(1) == "数詞"
            && self.pos(0) == "接尾辞"
            && self.pos(1) == "名詞的"
            && self.pos(2) == "助数詞"
            && previous.adjacent(self)
    }

    fn small_tsu_suffix(&self) -> bool {
        self.pos(0) == "接尾辞" && self.token.surface.starts_with(['っ', 'ッ', 'ｯ'])
    }
}

pub(super) fn standalone_small_tsu(surface: &str) -> bool {
    matches!(surface, "っ" | "ッ" | "ｯ")
}

/// Called separately for each detector sentence, preserving gaps and boundaries.
pub(super) fn join(
    morphemes: &[Morpheme],
    for_import: bool,
) -> impl Iterator<Item = (AnalyzedToken, ImportTokenContext)> + '_ {
    let mut index = 0;
    std::iter::from_fn(move || {
        let head = morphemes.get(index)?;
        let mut end = index + 1;
        let mut grammatical = index;
        let mut suffix = None;
        let mut preserve_join = false;
        let counted = morphemes
            .get(end)
            .is_some_and(|next| next.counter_after(head));
        if counted {
            preserve_join =
                for_import && head.token.is_japanese && morphemes[end].small_tsu_suffix();
            end += 1;
        }
        let mut pending_ending = false;
        while let Some(next) = morphemes.get(end) {
            if !morphemes[end - 1].adjacent(next) {
                break;
            }
            if for_import && head.token.is_japanese {
                if standalone_small_tsu(&next.token.surface) {
                    preserve_join = true;
                    end += 1;
                    continue;
                }
                if next.small_tsu_suffix() {
                    preserve_join = true;
                    suffix = Some(end);
                    grammatical = end;
                    end += 1;
                    continue;
                }
            }
            if !head.head() && suffix.is_none() {
                break;
            }
            let previous = &morphemes[grammatical];
            if next.ending_after(previous) {
                grammatical = end;
                end += 1;
            } else if previous.continuative() && next.connector() {
                let small_tsu_connector = for_import
                    && previous.pos(0) == "動詞"
                    && (previous.pos(5) == "連用形-促音便" || grammatical < end - 1)
                    && next.token.surface == "て";
                if morphemes
                    .get(end + 1)
                    .is_some_and(|helper| next.adjacent(helper) && helper.helper())
                {
                    preserve_join |= small_tsu_connector;
                    grammatical = end + 1;
                    end += 2;
                } else if small_tsu_connector {
                    preserve_join = true;
                    grammatical = end;
                    end += 1;
                } else {
                    pending_ending = true;
                    break;
                }
            } else {
                break;
            }
        }
        let mut token = head.token.clone();
        for (part_index, part) in (index + 1..end).zip(&morphemes[index + 1..end]) {
            if suffix == Some(part_index) {
                token.dictionary_form = part.token.dictionary_form.as_ref().and_then(|base| {
                    Surface::new(format!("{}{}", token.surface, base.as_str())).ok()
                });
            }
            token.surface.push_str(&part.token.surface);
            token.anchor.range.end = part.token.anchor.range.end;
            token.is_japanese |= part.token.is_japanese;
            token.reading =
                token
                    .reading
                    .zip(part.token.reading.as_ref())
                    .map(|(mut reading, suffix)| {
                        reading.push_str(suffix);
                        reading
                    });
        }
        if counted && suffix.is_none() {
            // A numeral's lemma would add unrelated number-only lookup results.
            token.dictionary_form = Surface::new(&token.surface).ok();
        }
        let lemma_head = suffix.unwrap_or(index);
        let lemma = &morphemes[lemma_head];
        let context = ImportTokenContext {
            starts_sentence: index == 0,
            standalone_grammar: matches!(head.pos(0), "助詞" | "助動詞"),
            inflected: lemma.conjugates()
                && (grammatical > lemma_head
                    || !matches!(lemma.pos(5), "終止形-一般" | "連体形-一般")),
            pending_ending,
            preserve_join,
        };
        index = end;
        Some((token, context))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use yomoku_core::{
        document::{TextAnchor, TextOffset, TextRange},
        vocabulary::Surface,
    };

    fn tokens(morphemes: Vec<Morpheme>) -> Vec<AnalyzedToken> {
        join(&morphemes, false).map(|(token, _)| token).collect()
    }

    #[test]
    fn counter_joining_requires_both_pos_tags_and_exact_adjacency() {
        let numeral = ["名詞", "数詞", "*", "*", "*", "*"];
        let noun = ["名詞", "普通名詞", "一般", "*", "*", "*"];
        let counter = ["接尾辞", "名詞的", "助数詞", "*", "*", "*"];
        let suffix = ["接尾辞", "名詞的", "一般", "*", "*", "*"];
        let morpheme = |surface: &str, start: i64, pos: [&str; 6]| Morpheme {
            token: AnalyzedToken {
                anchor: TextAnchor {
                    source_id: 1.try_into().unwrap(),
                    chapter_id: 1.try_into().unwrap(),
                    range: TextRange {
                        start: start.try_into().unwrap(),
                        end: (start + surface.len() as i64).try_into().unwrap(),
                    },
                },
                surface: surface.into(),
                is_japanese: true,
                reading: Some(surface.into()),
                dictionary_form: Some(Surface::new(surface).unwrap()),
            },
            pos: pos.map(str::to_owned).to_vec(),
            normalized_form: surface.into(),
        };
        for (head, tail, gap, expected) in [
            (numeral, counter, 0, 1),
            (noun, counter, 0, 2),
            (numeral, suffix, 0, 2),
            (numeral, counter, 1, 2),
        ] {
            let joined = tokens(vec![
                morpheme("ひと", 0, head),
                morpheme("つ", "ひと".len() as i64 + gap, tail),
            ]);
            assert_eq!(joined.len(), expected);
            if expected == 1 {
                assert_eq!(joined[0].reading.as_deref(), Some("ひとつ"));
                assert_eq!(
                    joined[0].dictionary_form.as_ref().unwrap().as_str(),
                    "ひとつ"
                );
            }
        }

        let counted = [
            morpheme("一", 0, numeral),
            morpheme("ッ", "一".len() as i64, counter),
        ];
        let mut joined = join(&counted, true);
        let (token, context) = joined.next().unwrap();
        assert_eq!(token.surface, "一ッ");
        assert!(context.preserve_join);
        assert!(joined.next().is_none());
    }

    #[test]
    fn joined_reading_requires_every_part_and_base_form_belongs_to_head() {
        for (head_reading, ending_reading) in [
            (None, Some("タ")),
            (Some("タベ"), None),
            (None, None),
            (Some("タベ"), Some("タ")),
        ] {
            let morpheme =
                |surface: &str, start, end, pos: &str, base: &str, reading: Option<&str>| {
                    Morpheme {
                        token: AnalyzedToken {
                            anchor: TextAnchor {
                                source_id: 1.try_into().unwrap(),
                                chapter_id: 1.try_into().unwrap(),
                                range: TextRange {
                                    start: TextOffset::try_from(start).unwrap(),
                                    end: TextOffset::try_from(end).unwrap(),
                                },
                            },
                            surface: surface.into(),
                            is_japanese: true,
                            reading: reading.map(str::to_owned),
                            dictionary_form: Some(Surface::new(base).unwrap()),
                        },
                        pos: [pos, "*", "*", "*", "conjugation", "連用形-一般"]
                            .map(str::to_owned)
                            .to_vec(),
                        normalized_form: base.into(),
                    }
                };
            let output = tokens(vec![
                morpheme("食べ", 0i64, 6i64, "動詞", "食べる", head_reading),
                morpheme("た", 6, 9, "助動詞", "た", ending_reading),
            ]);
            assert_eq!(output.len(), 1);
            assert_eq!(output[0].surface, "食べた");
            assert_eq!(
                output[0].dictionary_form.as_ref().unwrap().as_str(),
                "食べる"
            );
            assert_eq!(i64::from(output[0].anchor.range.end), 9);
            assert_eq!(
                output[0].reading.as_deref(),
                if head_reading.is_some() && ending_reading.is_some() {
                    Some("タベタ")
                } else {
                    None
                }
            );
        }
    }
}
