use yomoku_core::{
    anki::AnkiNote,
    dictionary::*,
    document::{ChapterText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectiveTokenEligibility,
    effective_session::{
        EffectivePageIdentity, EffectivePageToken, EffectiveReaderPage, EffectiveReaderSelection,
        EffectiveWordDetails,
    },
    reader::*,
    vocabulary::VocabularyStatus,
};

fn range(text: &str, start: usize, end: usize) -> TextRange {
    TextRange {
        start: TextOffset::at(text, start).unwrap(),
        end: TextOffset::at(text, end).unwrap(),
    }
}

struct Fixture {
    chapter: ChapterText,
    page: ReaderTokenSnapshot,
    details: ReaderWordDetails,
}

impl Fixture {
    fn new(text: &str, surface: &str) -> Self {
        let chapter = ChapterText {
            source_id: 1.try_into().unwrap(),
            chapter_id: 2.try_into().unwrap(),
            text: text.into(),
        };
        let page = TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: range(text, 0, text.len()),
        };
        let start = text.rfind(surface).unwrap();
        let token = ReaderToken {
            token_index: 7,
            anchor: TextAnchor {
                range: range(text, start, start + surface.len()),
                ..page
            },
            display_range: range(text, start, start + surface.len()),
            surface: surface.into(),
            reading: Some("アナライザー".into()),
            dictionary_form: None,
            vocabulary: Some(ReaderVocabulary {
                id: 3.try_into().unwrap(),
                status: VocabularyStatus::Learning,
            }),
        };
        let page = ReaderTokenSnapshot {
            document_id: 4.try_into().unwrap(),
            page,
            analysis: ReaderAnalysis::Available(5.try_into().unwrap()),
            tokens: vec![token.clone()],
        };
        let details = ReaderWordDetails {
            key: page.key(),
            token,
            encountered: 47,
            candidates: DictionaryCandidates {
                surface: surface.into(),
                dictionary_form: None,
                words: vec![],
                names: vec![],
            },
            custom_entry: None,
        };
        Self {
            chapter,
            page,
            details,
        }
    }

    fn original(&self) -> Result<AnkiNote, yomoku_core::document::InvalidTextValue> {
        AnkiNote::from_original_selection(
            &self.chapter,
            &self.page,
            self.details.selection(),
            &self.details,
        )
    }

    fn effective(&self) -> (EffectiveReaderPage, EffectiveWordDetails) {
        let original = &self.page.tokens[0];
        let token = EffectivePageToken {
            segment_index: 19,
            anchor: original.anchor,
            display_range: original.display_range,
            surface: original.surface.clone(),
            eligibility: EffectiveTokenEligibility::ManualJapaneseSpan,
            reading: original.reading.clone(),
            dictionary_form: original.dictionary_form.clone(),
            vocabulary: original.vocabulary,
        };
        let page = EffectiveReaderPage {
            identity: EffectivePageIdentity {
                revision: 11,
                base: self.page.key(),
            },
            tokens: vec![token.clone()],
            original_boundaries: vec![],
        };
        let details = EffectiveWordDetails {
            selection: EffectiveReaderSelection {
                page: page.identity,
                segment_index: token.segment_index,
            },
            token,
            encountered: self.details.encountered,
            candidates: self.details.candidates.clone(),
            custom_entry: self.details.custom_entry.clone(),
        };
        (page, details)
    }
}

fn spelling(text: &str) -> Spelling {
    Spelling {
        text: text.into(),
        ke_inf: vec![],
        ke_pri: vec![],
    }
}
fn reading(text: &str, restricted: &[&str]) -> Reading {
    Reading {
        text: text.into(),
        no_kanji: false,
        re_restr: restricted.iter().map(|s| (*s).into()).collect(),
        re_inf: vec![],
        re_pri: vec![],
    }
}
fn translation(text: &str, lang: &str) -> Translation {
    Translation {
        text: text.into(),
        lang: lang.into(),
        g_type: None,
        g_gend: None,
    }
}
fn sense(text: &str, spellings: &[&str], readings: &[&str]) -> Sense {
    Sense {
        stagk: spellings.iter().map(|s| (*s).into()).collect(),
        stagr: readings.iter().map(|s| (*s).into()).collect(),
        pos: vec![],
        field: vec![],
        misc: vec![],
        s_inf: vec![],
        dial: vec![],
        xref: vec![],
        ant: vec![],
        loan_sources: vec![],
        glosses: vec![translation(text, "eng")],
    }
}

fn candidates() -> DictionaryCandidates {
    let mut entry = WordEntry {
        kanji: vec![spelling("生"), spelling("他")],
        readings: vec![
            reading("せい", &[]),
            reading("しょう", &["生"]),
            reading("ほか", &["他"]),
        ],
        senses: vec![
            sense("life", &[], &[]),
            sense("birth", &["生"], &["しょう"]),
            sense("excluded", &["他"], &[]),
        ],
    };
    entry.senses[0].glosses.push(translation("vie", "fre"));
    entry.senses[0].glosses.push(Translation {
        g_type: Some("literal".into()),
        g_gend: Some("neutral".into()),
        ..translation("living", "eng")
    });
    let forms = vec![WordFormMatch {
        form: "生".into(),
        matches: entry.matches("生"),
    }];
    let name = NameEntry {
        kanji: vec![spelling("生")],
        readings: vec![reading("しょう", &[]), reading("いく", &[])],
        names: vec![NameGroup {
            name_type: vec!["given name".into()],
            xref: vec![],
            translations: vec![translation("Iku", "eng"), translation("non-English", "ger")],
        }],
    };
    let names = vec![NameCandidate {
        entry_id: 1,
        forms: vec![NameFormMatch {
            form: "生".into(),
            matches: matching_readings(&name.kanji, &name.readings, "生"),
        }],
        entry: name,
    }];
    DictionaryCandidates {
        surface: "生".into(),
        dictionary_form: None,
        words: vec![WordCandidate {
            entry_id: 1,
            entry,
            forms,
        }],
        names,
    }
}

#[test]
fn original_and_effective_export_all_licensed_readings_and_ordered_english_senses() {
    let mut f = Fixture::new("生と生。", "生");
    f.details.candidates = candidates();
    let before = (f.page.clone(), f.details.clone());
    let original = f.original().unwrap();
    let (page, details) = f.effective();
    let effective =
        AnkiNote::from_effective_selection(&f.chapter, &page, details.selection, &details).unwrap();
    assert_eq!(original, effective);
    assert_eq!(original.fields()[1], "せい、しょう、いく");
    assert_eq!(
        original.fields()[2],
        "<ol><li>life; living (literal, neutral)</li><li>birth</li><li>Iku</li></ol>"
    );
    assert_eq!(
        original.fields()[3],
        "生と<strong class=\"yomoku-target\">生</strong>。"
    );
    assert_eq!(before, (f.page.clone(), f.details.clone()));
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        f.page.tokens[0].vocabulary.as_mut().unwrap().status = status;
        assert_eq!(f.original().unwrap(), original);
        assert_eq!(f.page.tokens[0].vocabulary.unwrap().status, status);
    }
    f.page.tokens[0].vocabulary = None;
    assert_eq!(f.original().unwrap(), original); // Deleted vocabulary is not recreated.
}

#[test]
fn dictionary_form_supplies_definitions_without_replacing_inflected_surface() {
    let mut f = Fixture::new("生じた。", "生じた");
    f.page.tokens[0].dictionary_form = Some("生".into());
    f.details.token = f.page.tokens[0].clone();
    f.details.candidates = candidates();
    f.details.candidates.surface = "生じた".into();
    f.details.candidates.dictionary_form = Some("生".into());
    let note = f.original().unwrap();
    assert_eq!(note.surface().as_str(), "生じた");
    assert_eq!(note.fields()[0], "生じた");
    assert_eq!(note.fields()[1], "せい、しょう、いく");
}

#[test]
fn shared_surface_and_base_matches_do_not_repeat_readings_or_senses() {
    let mut f = Fixture::new("生。", "生");
    f.page.tokens[0].dictionary_form = Some("しょう".into());
    f.details.token = f.page.tokens[0].clone();
    f.details.candidates = candidates();
    f.details.candidates.dictionary_form = Some("しょう".into());
    let word = &mut f.details.candidates.words[0];
    word.forms.push(WordFormMatch {
        form: "しょう".into(),
        matches: word.entry.matches("しょう"),
    });
    let name = &mut f.details.candidates.names[0];
    name.forms.push(NameFormMatch {
        form: "しょう".into(),
        matches: matching_readings(&name.entry.kanji, &name.entry.readings, "しょう"),
    });
    let note = f.original().unwrap();
    assert_eq!(note.fields()[1], "せい、しょう、いく");
    assert_eq!(
        note.fields()[2],
        "<ol><li>life; living (literal, neutral)</li><li>birth</li><li>Iku</li></ol>"
    );
}

#[test]
fn unmatched_analyzer_reading_is_used_only_without_licensed_dictionary_readings() {
    let f = Fixture::new("猫。", "猫");
    assert_eq!(f.original().unwrap().fields()[1], "アナライザー");
    assert_eq!(f.original().unwrap().fields()[2], "");
    let mut f = Fixture::new("生。", "生");
    f.details.candidates = candidates();
    f.details.candidates.words.clear();
    f.details.candidates.names[0].entry.names[0].translations =
        vec![translation("non-English", "fre")];
    let note = f.original().unwrap();
    assert_eq!(note.fields()[1], "しょう、いく");
    assert_eq!(note.fields()[2], "");
}

#[test]
fn sentence_uses_full_chapter_when_page_only_displays_part_of_word() {
    let mut f = Fixture::new("前。猫と𠮷野家です。次。", "𠮷野家");
    let start = f.chapter.text.find('野').unwrap();
    let end = start + "野".len();
    f.page.page.range = range(&f.chapter.text, start, end);
    f.page.tokens[0].display_range = range("野", 0, "野".len());
    f.details.key = f.page.key();
    f.details.token = f.page.tokens[0].clone();
    assert_eq!(
        f.original().unwrap().fields()[3],
        "猫と<strong class=\"yomoku-target\">𠮷野家</strong>です。"
    );
    let (page, details) = f.effective();
    assert_eq!(
        AnkiNote::from_effective_selection(&f.chapter, &page, details.selection, &details).unwrap(),
        f.original().unwrap()
    );
}

#[test]
fn sentence_boundaries_preserve_quotes_hard_lines_whitespace_and_ascii_decimals() {
    for (text, surface, expected) in [
        (
            "前。「猫！？」次。",
            "猫",
            "「<strong class=\"yomoku-target\">猫</strong>！？」",
        ),
        (
            "前\r\n  猫と犬\r\n次",
            "猫",
            "  <strong class=\"yomoku-target\">猫</strong>と犬\r\n",
        ),
        (
            "前\r猫\r次",
            "猫",
            "<strong class=\"yomoku-target\">猫</strong>\r",
        ),
        (
            "前. 猫は1.5匹. 次",
            "猫",
            " <strong class=\"yomoku-target\">猫</strong>は1.5匹.",
        ),
        (
            "猫と猫",
            "猫",
            "猫と<strong class=\"yomoku-target\">猫</strong>",
        ),
        (
            "猫。犬。次",
            "猫。犬",
            "<strong class=\"yomoku-target\">猫。犬</strong>。",
        ),
    ] {
        let f = Fixture::new(text, surface);
        assert_eq!(f.original().unwrap().fields()[3], expected, "{text:?}");
    }
}

#[test]
fn custom_manual_spans_preserve_literal_data_and_absent_definitions_stay_empty() {
    let mut f = Fixture::new("前。か\u{3099}<猫>。", "か\u{3099}<猫>");
    f.page.tokens[0].reading = None;
    f.details.token = f.page.tokens[0].clone();
    let note = f.original().unwrap();
    assert_eq!(note.fields()[1], "");
    assert_eq!(note.fields()[2], "");
    f.details.custom_entry = Some(ReaderCustomEntry {
        surface: f.details.token.surface.clone(),
        english_meaning: " literal & [sound:a]\u{1f} ".into(),
        reading: " かな\n ".into(),
    });
    let (page, details) = f.effective();
    let note =
        AnkiNote::from_effective_selection(&f.chapter, &page, details.selection, &details).unwrap();
    assert_eq!(note.surface().as_str(), "か\u{3099}<猫>");
    assert_eq!(note.fields()[1], " かな\n ");
    assert_eq!(
        note.fields()[2],
        "<ol><li> literal &amp; &#91;sound:a&#93;&#31; </li></ol>"
    );
    assert_eq!(
        note.fields()[3],
        "<strong class=\"yomoku-target\">か\u{3099}&lt;猫&gt;</strong>。"
    );
}

#[test]
fn stale_original_keys_tokens_and_canonical_ownership_are_rejected() {
    for mutation in 0..10 {
        let mut f = Fixture::new("猫と猫。", "猫");
        match mutation {
            0 => f.page.document_id = 99.try_into().unwrap(),
            1 => f.page.analysis = ReaderAnalysis::Available(99.try_into().unwrap()),
            2 => f.page.analysis = ReaderAnalysis::Missing,
            3 => f.page.tokens[0].token_index += 1,
            4 => f.page.tokens[0].reading = Some("changed".into()),
            5 => f.page.tokens[0].dictionary_form = Some("犬".into()),
            6 => f.chapter.source_id = 99.try_into().unwrap(),
            7 => f.chapter.chapter_id = 99.try_into().unwrap(),
            8 => f.chapter.text = "犬と犬。".into(),
            9 => {
                f.details.token.display_range.start = TextOffset::START;
                f.page.tokens[0] = f.details.token.clone();
            }
            _ => unreachable!(),
        }
        assert!(f.original().is_err(), "mutation {mutation}");
    }
}

#[test]
fn stale_effective_revisions_selections_and_metadata_are_rejected() {
    let f = Fixture::new("猫。", "猫");
    for mutation in 0..5 {
        let (mut page, details) = f.effective();
        match mutation {
            0 => page.identity.revision += 1,
            1 => page.tokens[0].segment_index += 1,
            2 => page.tokens[0].surface = "犬".into(),
            3 => page.tokens[0].reading = None,
            4 => page.tokens[0].eligibility = EffectiveTokenEligibility::AnalyzerJapanese,
            _ => unreachable!(),
        }
        assert!(
            AnkiNote::from_effective_selection(&f.chapter, &page, details.selection, &details)
                .is_err()
        );
    }
}

#[test]
fn malformed_or_unrelated_dictionary_indices_and_custom_fallbacks_fail_closed() {
    for mutation in 0..9 {
        let mut f = Fixture::new("生。", "生");
        f.details.candidates = candidates();
        match mutation {
            0 => f.details.candidates.surface = "他".into(),
            1 => f.details.candidates.dictionary_form = Some("他".into()),
            2 => {
                f.details.candidates.words[0].forms[0].matches[0]
                    .reading
                    .reading_index = usize::MAX
            }
            3 => f.details.candidates.words[0].forms[0].matches[0]
                .sense_indices
                .push(usize::MAX),
            4 => f.details.candidates.words[0].forms[0].form = "他".into(),
            5 => f.details.candidates.names[0].forms[0].matches[0].reading_index = usize::MAX,
            6 => f.details.candidates.words[0].forms.clear(),
            7 => f.details.candidates.names[0].entry_id = 0,
            8 => {
                f.details.custom_entry = Some(ReaderCustomEntry {
                    surface: "生".into(),
                    english_meaning: "override".into(),
                    reading: "override".into(),
                })
            }
            _ => unreachable!(),
        }
        assert!(f.original().is_err(), "mutation {mutation}");
    }
    for (surface, meaning, reading) in [
        ("犬", "meaning", "よみ"),
        ("猫", " ", "よみ"),
        ("猫", "meaning", "\n"),
    ] {
        let mut f = Fixture::new("猫。", "猫");
        f.details.custom_entry = Some(ReaderCustomEntry {
            surface: surface.into(),
            english_meaning: meaning.into(),
            reading: reading.into(),
        });
        assert!(f.original().is_err());
    }
}
