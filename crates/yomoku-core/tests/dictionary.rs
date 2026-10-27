use yomoku_core::dictionary::{Reading, Sense, Spelling, Translation, WordEntry};

#[test]
fn custom_entry_requires_both_fields_and_preserves_literal_unicode() {
    use yomoku_core::{dictionary::CustomDictionaryEntry, vocabulary::Surface};

    let surface = Surface::new("𠮷か\u{3099}\0").unwrap();
    for (meaning, reading) in [
        ("", "かな"),
        (" \n\t", "かな"),
        ("meaning", ""),
        ("meaning", "　\n"),
    ] {
        assert!(
            CustomDictionaryEntry::new(surface.clone(), meaning.into(), reading.into()).is_err()
        );
    }
    let entry = CustomDictionaryEntry::new(
        surface.clone(),
        " literal\nmeaning 📖\0 ".into(),
        " か\u{3099}𠮷\0 ".into(),
    )
    .unwrap();
    assert_eq!(entry.surface(), &surface);
    assert_eq!(entry.english_meaning(), " literal\nmeaning 📖\0 ");
    assert_eq!(entry.reading(), " か\u{3099}𠮷\0 ");
}

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
