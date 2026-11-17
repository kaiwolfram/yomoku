//! Production Mode C post-processing; A/B output stays raw for dictionary lookup.

use yomoku_core::{analysis::AnalyzedToken, vocabulary::Surface};

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

    fn ending_after(&self, previous: &Self) -> bool {
        self.token.is_japanese
            && self.conjugates()
            && previous.conjugates()
            && (self.pos(0) == "助動詞"
                || (self.pos(0) == "形容詞"
                    && self.pos(1) == "非自立可能"
                    && self.normalized_form == "無い"
                    && previous.pos(0) == "形容詞"
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
}

/// Called separately for each detector sentence: neither source gaps nor sentence
/// boundaries can be swallowed. A connector is consumed only with a valid helper.
pub(super) fn join(morphemes: Vec<Morpheme>) -> Vec<AnalyzedToken> {
    let mut output = Vec::with_capacity(morphemes.len());
    let mut index = 0;
    while index < morphemes.len() {
        let head = &morphemes[index];
        let mut end = index + 1;
        let counted = morphemes
            .get(end)
            .is_some_and(|next| next.counter_after(head));
        if counted {
            end += 1;
        } else if head.head() {
            while end < morphemes.len() {
                let previous = &morphemes[end - 1];
                let next = &morphemes[end];
                if !previous.adjacent(next) {
                    break;
                }
                if next.ending_after(previous) {
                    end += 1;
                } else if previous.continuative()
                    && next.connector()
                    && morphemes
                        .get(end + 1)
                        .is_some_and(|helper| next.adjacent(helper) && helper.helper())
                {
                    end += 2;
                } else {
                    break;
                }
            }
        }
        let mut token = head.token.clone();
        for part in &morphemes[index + 1..end] {
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
        if counted {
            // A numeral's lemma would add unrelated number-only lookup results.
            token.dictionary_form = Surface::new(&token.surface).ok();
        }
        output.push(token);
        index = end;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use yomoku_core::{
        document::{TextAnchor, TextOffset, TextRange},
        vocabulary::Surface,
    };

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
            let joined = join(vec![
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
            let output = join(vec![
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
