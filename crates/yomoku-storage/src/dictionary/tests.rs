use super::*;
use crate::tests::TestDatabase;
use serde_json::{json, Value};
use std::fs;

#[test]
fn part_discovery_uses_forms_only_orders_maximal_matches_and_keeps_exact_occurrences() {
    let file = fixture();
    let connection = Connection::open(file.path()).unwrap();
    for (id, text) in [
        (10, "ふくろう"),
        (11, "便"),
        (12, "ふく"),
        (13, "くろう"),
        (14, "う"),
        (15, "食べる"),
        (16, "𠮷"),
    ] {
        insert(
            &connection,
            "JMdict",
            id,
            &json!({"kanji": [], "readings": [{"text": text}], "senses": [{"glosses": [{"text": text, "lang": "eng"}]}]}),
        );
    }
    // Discovery must not decode an unselected entry's payload.
    connection
        .execute(
            "UPDATE entries SET payload = 'invalid' WHERE entry_id = 12",
            [],
        )
        .unwrap();
    let dictionary = Dictionary::open(file.path()).unwrap();
    assert!(dictionary
        .lookup("ふくろう便", None)
        .unwrap()
        .words
        .is_empty());
    let parts = dictionary.parts("ふくろう便", vec![]).unwrap();
    assert_eq!(
        parts
            .iter()
            .filter(|part| part.primary)
            .map(|part| part.surface.as_str())
            .collect::<Vec<_>>(),
        ["ふくろう", "便"]
    );
    assert!(parts
        .iter()
        .any(|part| part.surface == "くろう" && !part.primary));
    for part in parts.iter().filter(|part| part.primary) {
        let found = dictionary
            .lookup(&part.surface, part.dictionary_form.as_deref())
            .unwrap();
        assert_eq!(found.words.len(), 1);
        assert_eq!(part.range.slice("ふくろう便").unwrap(), part.surface);
    }
    let repeated = dictionary.parts("😀𠮷𠮷", vec![]).unwrap();
    assert_eq!(repeated.len(), 1);
    assert_eq!(i64::from(repeated[0].range.start), 4);
    assert_eq!(i64::from(repeated[0].range.end), 8);
    let names = dictionary.parts("未試験語末", vec![]).unwrap();
    assert!(names.iter().any(|part| part.surface == "試験語"));
    assert!(!dictionary.lookup("試験語", None).unwrap().names.is_empty());
    assert!(dictionary
        .parts(&"龘".repeat(10_000), vec![])
        .unwrap()
        .is_empty());
    assert!(dictionary.parts("xyz", vec![]).unwrap().is_empty());
}

#[test]
fn component_suggestions_retain_supplied_inflections_and_earliest_base_provenance() {
    use yomoku_core::{dictionary::DictionaryPart, document::TextRange};
    let file = fixture();
    let dictionary = Dictionary::open(file.path()).unwrap();
    let part = DictionaryPart {
        surface: "試験語だった".into(),
        dictionary_form: Some("試験語".into()),
        range: TextRange {
            start: 0.try_into().unwrap(),
            end: 18.try_into().unwrap(),
        },
        primary: false,
    };
    let parts = dictionary
        .parts(
            "試験語だった試験語だった",
            vec![
                DictionaryPart {
                    range: TextRange {
                        start: 18.try_into().unwrap(),
                        end: 36.try_into().unwrap(),
                    },
                    ..part.clone()
                },
                part.clone(),
            ],
        )
        .unwrap();
    let first = parts
        .iter()
        .find(|value| value.surface == part.surface)
        .unwrap();
    assert_eq!(first.range, part.range);
    assert_eq!(first.dictionary_form, part.dictionary_form);
    assert!(first.primary);
    let found = dictionary
        .lookup(&first.surface, first.dictionary_form.as_deref())
        .unwrap();
    assert!(!found.words.is_empty() && !found.names.is_empty());
    assert!(dictionary
        .lookup("試験語だった試験語だった", None)
        .unwrap()
        .words
        .is_empty());
}

#[test]
#[ignore = "requires prepared full dictionary index"]
fn bundled_fukurou_bin_suggests_separate_parts_without_whole_word_definitions() {
    let dictionary = Dictionary::open(std::env::var("YOMOKU_DICTIONARY_INDEX").unwrap()).unwrap();
    let whole = dictionary.lookup("ふくろう便", None).unwrap();
    assert!(whole.words.is_empty() && whole.names.is_empty());
    let parts = dictionary.parts("ふくろう便", vec![]).unwrap();
    assert_eq!(
        parts
            .iter()
            .filter(|part| part.primary)
            .map(|part| part.surface.as_str())
            .collect::<Vec<_>>(),
        ["ふくろう", "便"]
    );
    for part in parts.iter().filter(|part| part.primary) {
        assert!(!dictionary
            .lookup(&part.surface, None)
            .unwrap()
            .words
            .is_empty());
    }
}

fn word() -> Value {
    json!({
        "kanji": [{"text": "試験語", "ke_inf": ["test spelling"], "ke_pri": ["spec1"]}, {"text": "試験語𠮷"}],
        "readings": [{"text": "しけんご", "re_restr": ["試験語"], "re_inf": ["test reading"], "re_pri": ["spec1"]}, {"text": "テスト", "no_kanji": true}],
        "senses": [
            {"pos": ["noun"], "glosses": [{"text": "synthetic", "lang": "eng"}, {"text": "literal", "lang": "eng", "g_type": "lit", "g_gend": "neut"}]},
            {"stagk": ["試験語𠮷"], "stagr": ["テスト"], "pos": ["noun"], "field": ["testing"], "misc": ["rare"], "s_inf": ["explanation"], "dial": ["test dialect"], "xref": ["別語"], "ant": ["反語"],
             "loan_sources": [{"text": "", "lang": "fre", "type": "part", "wasei": true}], "glosses": [{"text": "restricted", "lang": "eng"}]}
        ]
    })
}

fn name() -> Value {
    json!({"kanji": [{"text": "試験語"}], "readings": [{"text": "しけんご"}],
        "names": [{"name_type": ["family or surname"], "xref": ["別名"], "translations": [{"text": "Test Name", "lang": "eng"}, {"text": "Other Name", "lang": "eng"}]},
                  {"name_type": ["place name"], "translations": [{"text": "Test Place", "lang": "eng"}]}]})
}

fn insert(connection: &Connection, source: &str, id: i64, payload: &Value) {
    connection
        .execute(
            "INSERT INTO entries VALUES (?1, ?2, ?3)",
            (source, id, payload.to_string()),
        )
        .unwrap();
    for field in ["kanji", "readings"] {
        for form in payload[field].as_array().unwrap() {
            connection
                .execute(
                    "INSERT OR IGNORE INTO forms VALUES (?1, ?2, ?3)",
                    (form["text"].as_str().unwrap(), source, id),
                )
                .unwrap();
        }
    }
}

fn fixture() -> TestDatabase {
    let file = TestDatabase::new();
    let connection = Connection::open(file.path()).unwrap();
    connection.execute_batch("PRAGMA application_id = 0x594f4449; PRAGMA user_version = 1;
        CREATE TABLE entries (source TEXT, entry_id INTEGER, payload TEXT, PRIMARY KEY(source, entry_id)) WITHOUT ROWID;
        CREATE TABLE forms (surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER, PRIMARY KEY(surface, source, entry_id)) WITHOUT ROWID;").unwrap();
    insert(&connection, "JMdict", 1, &word());
    insert(&connection, "JMnedict", 1, &name());
    for (id, text) in [(2, "しけんご"), (3, "か\u{3099}"), (4, "試験語\0")] {
        insert(
            &connection,
            "JMdict",
            id,
            &json!({"kanji": [], "readings": [{"text": text}], "senses": [{"glosses": [{"text": "synthetic", "lang": "eng"}]}]}),
        );
    }
    file
}

#[test]
fn surface_base_union_preserves_identity_all_metadata_and_separate_sources() {
    let file = fixture();
    let db = Dictionary::open(file.path()).unwrap();
    let found = db.lookup("試験語", Some("しけんご")).unwrap();
    assert_eq!(found.surface, "試験語");
    assert_eq!(found.dictionary_form.as_deref(), Some("しけんご"));
    assert_eq!(
        found.words.iter().map(|w| w.entry_id).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(found.names[0].entry_id, 1);
    let word = &found.words[0];
    assert_eq!(word.forms.len(), 2);
    assert_eq!(word.forms[0].form, "試験語");
    assert_eq!(word.forms[1].form, "しけんご");
    assert_eq!(word.forms[0].matches[0].sense_indices, [0]);
    assert_eq!(word.entry.kanji[0].ke_inf, ["test spelling"]);
    assert_eq!(word.entry.kanji[0].ke_pri, ["spec1"]);
    assert_eq!(word.entry.readings[0].re_inf, ["test reading"]);
    assert_eq!(word.entry.readings[0].re_pri, ["spec1"]);
    assert_eq!(word.entry.readings[0].re_restr, ["試験語"]);
    assert!(word.entry.readings[1].no_kanji);
    let sense = &word.entry.senses[1];
    assert_eq!(sense.stagk, ["試験語𠮷"]);
    assert_eq!(sense.stagr, ["テスト"]);
    assert_eq!(sense.pos, ["noun"]);
    assert_eq!(sense.field, ["testing"]);
    assert_eq!(sense.misc, ["rare"]);
    assert_eq!(sense.s_inf, ["explanation"]);
    assert_eq!(sense.dial, ["test dialect"]);
    assert_eq!(sense.xref, ["別語"]);
    assert_eq!(sense.ant, ["反語"]);
    assert_eq!(sense.loan_sources[0].kind, "part");
    assert_eq!(sense.loan_sources[0].lang, "fre");
    assert!(sense.loan_sources[0].wasei);
    assert_eq!(
        word.entry.senses[0].glosses[1].g_type.as_deref(),
        Some("lit")
    );
    assert_eq!(
        word.entry.senses[0].glosses[1].g_gend.as_deref(),
        Some("neut")
    );
    let names = &found.names[0].entry.names;
    assert_eq!(names.len(), 2);
    assert_eq!(names[0].translations.len(), 2);
    assert_eq!(names[0].name_type, ["family or surname"]);
    assert_eq!(names[0].xref, ["別名"]);
    assert_eq!(names[1].name_type, ["place name"]);
    assert_eq!(
        db.lookup("試験語", Some("試験語")).unwrap().words[0]
            .forms
            .len(),
        1
    );
    let inflected = db.lookup("試験語だった", Some("試験語")).unwrap();
    assert_eq!(inflected.surface, "試験語だった");
    assert_eq!(inflected.words[0].forms[0].form, "試験語");
    assert!(db.lookup("unknown", None).unwrap().words.is_empty());
}

#[test]
fn queries_are_exact_and_parameterized_with_no_normalization_or_reading_guess() {
    let file = fixture();
    let db = Dictionary::open(file.path()).unwrap();
    for form in ["試験", "が", "シケンゴ", "' OR 1=1 --"] {
        let found = db.lookup(form, None).unwrap();
        assert!(found.words.is_empty());
        assert!(found.names.is_empty());
    }
    assert_eq!(db.lookup("か\u{3099}", None).unwrap().words[0].entry_id, 3);
    assert_eq!(db.lookup("試験語\0", None).unwrap().words[0].entry_id, 4);
    assert_eq!(db.lookup("試験語𠮷", None).unwrap().words.len(), 1);
    assert_eq!(db.lookup("しけんご", None).unwrap().words.len(), 2);
    for (surface, base) in [("", None), ("\n\t", None), ("試験語", Some(" "))] {
        assert!(matches!(
            db.lookup(surface, base),
            Err(DictionaryError::InvalidQuery)
        ));
    }
}

#[test]
fn read_only_lookup_reopen_and_query_plan_leave_database_bytes_unchanged() {
    let file = fixture();
    let before = fs::read(file.path()).unwrap();
    let db = Dictionary::open(file.path()).unwrap();
    assert!(db.connection.is_readonly("main").unwrap());
    assert!(db.connection.execute("DELETE FROM entries", []).is_err());
    let plan: Vec<String> = db
        .connection
        .prepare(&format!("EXPLAIN QUERY PLAN {LOOKUP_SQL}"))
        .unwrap()
        .query_map(["試験語"], |r| r.get(3))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(
        plan.iter()
            .any(|s| s.contains("SEARCH f USING PRIMARY KEY (surface=?)")),
        "{plan:?}"
    );
    let expected = db.lookup("試験語", Some("しけんご")).unwrap();
    drop(db);
    assert_eq!(
        Dictionary::open(file.path())
            .unwrap()
            .lookup("試験語", Some("しけんご"))
            .unwrap(),
        expected
    );
    assert_eq!(fs::read(file.path()).unwrap(), before);
    assert_eq!(
        fs::read_dir(file.path().parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn missing_wrong_identity_future_version_and_non_database_are_never_repaired() {
    let file = TestDatabase::new();
    assert!(Dictionary::open(file.path()).is_err());
    assert!(!file.path().exists());
    fs::write(file.path(), b"not a SQLite asset").unwrap();
    assert!(Dictionary::open(file.path()).is_err());
    assert_eq!(fs::read(file.path()).unwrap(), b"not a SQLite asset");
    let file = fixture();
    let connection = Connection::open(file.path()).unwrap();
    for sql in [
        "PRAGMA application_id = 0",
        "PRAGMA application_id = 0x594f4449; PRAGMA user_version = 2",
    ] {
        connection.execute_batch(sql).unwrap();
        let before = fs::read(file.path()).unwrap();
        assert!(matches!(
            Dictionary::open(file.path()),
            Err(DictionaryError::UnrecognizedIndex)
        ));
        assert_eq!(fs::read(file.path()).unwrap(), before);
    }
}

#[test]
fn malformed_unknown_and_inconsistent_payloads_fail_without_partial_results_or_writes() {
    let file = fixture();
    let connection = Connection::open(file.path()).unwrap();
    let mut bad_cases = vec!["{".into(), "{}".into()];
    for mutate in [
        |v: &mut Value| {
            v["future_field"] = json!(true);
        },
        |v: &mut Value| {
            v["readings"][0]["re_restr"] = json!(["absent"]);
        },
        |v: &mut Value| {
            v["senses"][0]["stagk"] = json!(["absent"]);
        },
        |v: &mut Value| {
            v["senses"][0]["stagr"] = json!(["absent"]);
        },
        |v: &mut Value| {
            v["senses"][0]["glosses"][0]["lang"] = json!("fre");
        },
        |v: &mut Value| {
            v["readings"] = json!([]);
        },
        |v: &mut Value| {
            v["senses"] = json!([]);
        },
    ] {
        let mut payload = word();
        mutate(&mut payload);
        bad_cases.push(payload.to_string());
    }
    for bad in bad_cases {
        connection
            .execute(
                "UPDATE entries SET payload = ?1 WHERE source = 'JMdict' AND entry_id = 1",
                [bad],
            )
            .unwrap();
        let before = fs::read(file.path()).unwrap();
        assert!(Dictionary::open(file.path())
            .unwrap()
            .lookup("試験語", None)
            .is_err());
        assert_eq!(fs::read(file.path()).unwrap(), before);
    }
    connection
        .execute(
            "UPDATE entries SET payload = ?1 WHERE source = 'JMdict' AND entry_id = 1",
            [word().to_string()],
        )
        .unwrap();
    connection
        .execute("INSERT INTO forms VALUES ('wrong form', 'JMdict', 1)", [])
        .unwrap();
    assert!(matches!(
        Dictionary::open(file.path())
            .unwrap()
            .lookup("wrong form", None),
        Err(DictionaryError::InvalidEntry)
    ));
    connection
        .execute(
            "UPDATE entries SET payload = '{}' WHERE source = 'JMnedict'",
            [],
        )
        .unwrap();
    assert!(Dictionary::open(file.path())
        .unwrap()
        .lookup("試験語", None)
        .is_err());
}

#[test]
#[ignore = "requires prepared full index; set YOMOKU_DICTIONARY_INDEX to dictionary.sqlite3"]
fn full_index_all_payloads_and_real_surface_base_word_name_matches() {
    let path = std::env::var_os("YOMOKU_DICTIONARY_INDEX").expect("set YOMOKU_DICTIONARY_INDEX");
    let db = Dictionary::open(path).unwrap();
    let mut statement = db
        .connection
        .prepare("SELECT source, payload FROM entries ORDER BY source, entry_id")
        .unwrap();
    let mut rows = statement.query([]).unwrap();
    let mut counts = [0; 2];
    while let Some(row) = rows.next().unwrap() {
        let source: String = row.get(0).unwrap();
        let payload: String = row.get(1).unwrap();
        if source == "JMdict" {
            let entry: WordEntry = serde_json::from_str(&payload).unwrap();
            let form = &entry.readings[0].text;
            validate_word(&entry, form).unwrap();
            counts[0] += 1;
        } else {
            assert_eq!(source, "JMnedict");
            let entry: NameEntry = serde_json::from_str(&payload).unwrap();
            validate_name(&entry, &entry.readings[0].text).unwrap();
            counts[1] += 1;
        }
    }
    assert_eq!(counts, [218_957, 743_682]);
    let eat = db.lookup("食べた", Some("食べる")).unwrap();
    assert_eq!(eat.surface, "食べた");
    assert!(eat
        .words
        .iter()
        .any(|w| w.entry.readings.iter().any(|r| r.text == "たべる")
            && !w.forms[0].matches.is_empty()));
    assert!(!db.lookup("たべる", None).unwrap().words.is_empty());
    assert!(!db.lookup("ありがとう", None).unwrap().words.is_empty());
    let tokyo = db.lookup("東京", None).unwrap();
    assert!(!tokyo.words.is_empty());
    assert!(!tokyo.names.is_empty());
    assert!(tokyo.names.iter().all(|n| !n.forms[0].matches.is_empty()));
    assert!(db
        .lookup("yomoku-no-dictionary-match-𠮷", None)
        .unwrap()
        .words
        .is_empty());
}

#[test]
fn component_queries_keep_full_token_first_and_deduplicate_entries_and_forms() {
    let file = fixture();
    let db = Dictionary::open(file.path()).unwrap();
    let found = db
        .lookup_forms(
            "か\u{3099}",
            None,
            &["試験語".into(), "しけんご".into(), "試験語".into()],
        )
        .unwrap();
    assert_eq!(
        found
            .words
            .iter()
            .map(|entry| entry.entry_id)
            .collect::<Vec<_>>(),
        [3, 1, 2]
    );
    assert_eq!(
        found.words[1]
            .forms
            .iter()
            .map(|form| form.form.as_str())
            .collect::<Vec<_>>(),
        ["試験語", "しけんご"]
    );
    assert_eq!(found.names.len(), 1);
    assert_eq!(found.names[0].forms.len(), 2);
}
