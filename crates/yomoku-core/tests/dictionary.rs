use yomoku_core::dictionary::{Reading, Sense, Spelling, Translation, WordEntry};

fn spelling(text: &str) -> Spelling {
    Spelling {
        text: text.into(),
        ke_inf: vec![],
        ke_pri: vec![],
    }
}

fn reading(text: &str, restrictions: &[&str], no_kanji: bool) -> Reading {
    Reading {
        text: text.into(),
        no_kanji,
        re_restr: restrictions.iter().map(|s| (*s).into()).collect(),
        re_inf: vec![],
        re_pri: vec![],
    }
}

fn sense(spellings: &[&str], readings: &[&str]) -> Sense {
    Sense {
        stagk: spellings.iter().map(|s| (*s).into()).collect(),
        stagr: readings.iter().map(|s| (*s).into()).collect(),
        pos: vec!["noun".into()],
        field: vec![],
        misc: vec![],
        s_inf: vec![],
        dial: vec![],
        xref: vec![],
        ant: vec![],
        loan_sources: vec![],
        glosses: vec![Translation {
            text: "synthetic".into(),
            lang: "eng".into(),
            g_type: None,
            g_gend: None,
        }],
    }
}

fn entry() -> WordEntry {
    WordEntry {
        kanji: vec![spelling("試験語"), spelling("試験語𠮷")],
        readings: vec![
            reading("しけんご", &[], false),
            reading("ためしご", &["試験語"], false),
            reading("テスト", &[], true),
        ],
        senses: vec![
            sense(&[], &[]),
            sense(&["試験語"], &["ためしご"]),
            sense(&["試験語𠮷"], &["しけんご"]),
            sense(&[], &["テスト"]),
        ],
    }
}

#[test]
fn written_query_keeps_all_readings_and_applies_both_sense_restrictions() {
    let entry = entry();
    let found = entry.matches("試験語");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].reading.reading_index, 0);
    assert_eq!(found[0].sense_indices, [0]);
    assert_eq!(found[1].reading.reading_index, 1);
    assert_eq!(found[1].sense_indices, [0, 1]);
    let other = entry.matches("試験語𠮷");
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].sense_indices, [0, 2]);
    assert_eq!(entry, self::entry()); // Selection does not edit the payload.
}

#[test]
fn kana_query_keeps_all_allowed_spellings_without_leaking_restricted_readings() {
    let entry = entry();
    let found = entry.matches("しけんご");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].reading.spelling_index, Some(0));
    assert_eq!(found[0].sense_indices, [0]);
    assert_eq!(found[1].reading.spelling_index, Some(1));
    assert_eq!(found[1].sense_indices, [0, 2]);
    let limited = entry.matches("ためしご");
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].sense_indices, [0, 1]);
}

#[test]
fn no_kanji_and_kana_only_do_not_satisfy_spelling_restrictions() {
    let mut entry = entry();
    let found = entry.matches("テスト");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reading.spelling_index, None);
    assert_eq!(found[0].sense_indices, [0, 3]);
    entry.kanji.clear();
    let found = entry.matches("しけんご");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reading.spelling_index, None);
    assert_eq!(found[0].sense_indices, [0]);
}

#[test]
fn selection_is_exact_including_kana_script_combining_and_nul() {
    let mut entry = entry();
    entry.readings.push(reading("か\u{3099}", &[], true));
    for form in ["シケンゴ", "試験", "試験語\0", "が", "' OR 1=1 --"] {
        assert!(entry.matches(form).is_empty(), "{form:?}");
    }
    assert_eq!(entry.matches("か\u{3099}").len(), 1);
}

#[test]
fn impossible_sense_pair_is_not_displayable_and_ambiguous_form_is_not_duplicated() {
    let mut entry = entry();
    entry.senses = vec![sense(&["試験語𠮷"], &["ためしご"])];
    assert!(entry.matches("試験語").is_empty());
    entry.senses = vec![sense(&[], &[])];
    entry.readings[0].text = "試験語".into();
    let found = entry.matches("試験語");
    assert_eq!(found.len(), 3); // Two spelling pairs for reading 0, one for reading 1.
    assert_ne!(found[0].reading, found[1].reading);
}

#[test]
fn components_keep_length_position_and_exact_form_highlight_ownership() {
    use yomoku_core::{
        analysis::AnalyzedToken,
        dictionary::component_sources,
        document::{TextAnchor, TextRange},
        vocabulary::Surface,
    };
    let token = |surface: &str, base: Option<&str>, start: i64, japanese| AnalyzedToken {
        anchor: TextAnchor {
            source_id: 1.try_into().unwrap(),
            chapter_id: 1.try_into().unwrap(),
            range: TextRange {
                start: start.try_into().unwrap(),
                end: (start + surface.len() as i64).try_into().unwrap(),
            },
        },
        surface: surface.into(),
        dictionary_form: base.map(|form| Surface::new(form).unwrap()),
        reading: None,
        is_japanese: japanese,
    };
    let early = token("食べた", Some("食べる"), 0, true);
    let late = token("食べた", Some("食べる"), 20, true);
    let sources = component_sources(vec![
        token("ignored-long-text", None, 0, false),
        token("猫", None, 30, true),
        late,
        token("東京都", None, 10, true),
        early.clone(),
        token("猫", None, 40, true),
    ]);
    assert_eq!(
        sources
            .iter()
            .map(|source| source.form.as_str())
            .collect::<Vec<_>>(),
        ["食べた", "食べる", "東京都", "猫"]
    );
    assert_eq!(sources[0].anchor, early.anchor);
    assert_eq!(sources[1].anchor, early.anchor);
    assert_eq!(i64::from(sources[3].anchor.range.start), 30);
}
