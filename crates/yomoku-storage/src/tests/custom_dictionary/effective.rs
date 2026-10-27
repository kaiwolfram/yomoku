use super::*;
use yomoku_core::{
    analysis::{AnalyzedToken, ChapterAnalysis, OverrideScope, SegmentationOverride, SplitMode},
    effective_reader::{EffectiveReaderTokenKey, EffectiveReaderTokenSnapshot},
};

fn snapshot(storage: &mut Storage, imported: &AnalyzedPaste) -> EffectiveReaderTokenSnapshot {
    storage
        .load_effective_reader_tokens(imported.document.document_id, key(imported).page)
        .unwrap()
        .unwrap()
}

fn split(storage: &mut Storage, imported: &AnalyzedPaste) -> yomoku_core::document::OverrideId {
    storage
        .save_segmentation_override(&SegmentationOverride {
            anchor: TextAnchor {
                range: TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(&imported.document.chapter.text, "造語だった".len())
                        .unwrap(),
                },
                ..key(imported).page
            },
            surface: Surface::new("造語だった").unwrap(),
            scope: OverrideScope::AllOccurrences,
            parts: ["造語", "だった"].map(|s| Surface::new(s).unwrap()).into(),
        })
        .unwrap()
}

fn save_effective(
    storage: &mut Storage,
    dictionary: &Dictionary,
    key: &EffectiveReaderTokenKey,
    index: u64,
    meaning: &str,
) -> CustomDictionaryEntry {
    storage
        .save_effective_reader_custom_entry(
            dictionary,
            key,
            index,
            meaning.into(),
            " よみ\u{3099}\0 ".into(),
        )
        .unwrap()
        .unwrap()
}

fn load_effective(
    storage: &mut Storage,
    dictionary: &Dictionary,
    key: &EffectiveReaderTokenKey,
    index: u64,
) -> Option<CustomDictionaryEntry> {
    storage
        .load_effective_reader_custom_entry(dictionary, key, index)
        .unwrap()
}

fn rejected(
    storage: &mut Storage,
    dictionary: &Dictionary,
    key: &EffectiveReaderTokenKey,
    index: u64,
    expected: fn(&CustomDictionaryError) -> bool,
) {
    let results = [
        storage
            .load_effective_reader_custom_entry(dictionary, key, index)
            .map(|_| ()),
        storage
            .save_effective_reader_custom_entry(
                dictionary,
                key,
                index,
                "changed".into(),
                "かな".into(),
            )
            .map(|_| ()),
        storage
            .delete_effective_reader_custom_entry(dictionary, key, index)
            .map(|_| ()),
    ];
    for result in results {
        let error = result.unwrap_err();
        assert!(expected(&error), "unexpected error: {error:?}");
    }
}

fn stale(error: &CustomDictionaryError) -> bool {
    matches!(
        error,
        CustomDictionaryError::Selection(ReaderWordError::InvalidSelection)
    )
}

#[test]
fn effective_custom_dictionary_crud_is_global_literal_and_preserves_reader_history_and_metadata() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語だった 造語だった");
    let other = import(&mut storage, "造語");
    let vocabulary = storage
        .vocabulary_entry(&Surface::new("造語").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(vocabulary.id, VocabularyStatus::Ignored, 110)
        .unwrap();
    split(&mut storage, &imported);
    let before = snapshot(&mut storage, &imported);
    let frozen = before.key().unwrap();
    let reader_before = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap();
    let index_before = fs::read(index.path()).unwrap();
    assert_eq!(load_effective(&mut storage, &dictionary, &frozen, 0), None);
    let entry = save_effective(
        &mut storage,
        &dictionary,
        &frozen,
        0,
        " literal\nmeaning 𠮷\0 ",
    );
    assert_eq!(
        load(&mut storage, &dictionary, key(&other)),
        Some(entry.clone())
    );
    assert_eq!(
        load_effective(&mut storage, &dictionary, &frozen, 2),
        Some(entry.clone())
    );
    let edited = save_effective(&mut storage, &dictionary, &frozen, 2, "edited");
    // New manual spellings remain unregistered, including after custom CRUD.
    let unregistered = save_effective(&mut storage, &dictionary, &frozen, 1, "was");
    assert_eq!(unregistered.surface().as_str(), "だった");
    assert!(storage
        .vocabulary_entry(&Surface::new("だった").unwrap())
        .unwrap()
        .is_none());
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(vocabulary.id)],
        )
        .unwrap();
    assert_eq!(
        load_effective(&mut storage, &dictionary, &frozen, 0),
        Some(edited.clone())
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("造語").unwrap())
        .unwrap()
        .is_none());
    let after = snapshot(&mut storage, &imported);
    assert_eq!(after.key(), before.key());
    assert!(after
        .tokens
        .iter()
        .all(|t| t.metadata.is_none() && t.vocabulary.is_none()));
    assert_eq!(
        storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap(),
        reader_before
    );
    let counts: (i64, i64, i64) = storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM vocabulary_status_events), (SELECT COUNT(*) FROM reading_occurrences), (SELECT COUNT(*) FROM reading_completions)", [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(counts, (1, 0, 0)); // Deleted vocabulary's own events cascade away.
    assert_eq!(fs::read(index.path()).unwrap(), index_before);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    assert_eq!(
        load_effective(&mut storage, &dictionary, &frozen, 0),
        Some(edited.clone())
    );
    assert_eq!(
        storage
            .load_effective_reader_word(&dictionary, &frozen, 2)
            .unwrap()
            .unwrap()
            .custom_entry,
        Some(edited.into())
    );
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    assert!(storage
        .delete_effective_reader_custom_entry(&dictionary, &frozen, 2)
        .unwrap());
    assert_eq!(load(&mut storage, &dictionary, key(&other)), None);
    assert!(!storage
        .delete_effective_reader_custom_entry(&dictionary, &frozen, 0)
        .unwrap());
}

#[test]
fn effective_custom_dictionary_protects_surface_word_name_and_exact_analyzer_base_matches() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    for text in ["既存語", "人名", "既存語だった"] {
        let imported = import(&mut storage, text);
        let frozen = snapshot(&mut storage, &imported).key().unwrap();
        let bytes = fs::read(database.path()).unwrap();
        rejected(&mut storage, &dictionary, &frozen, 0, |e| {
            matches!(e, CustomDictionaryError::BuiltInMatch)
        });
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
    let imported = import(&mut storage, "造語だった");
    split(&mut storage, &imported);
    let before = snapshot(&mut storage, &imported);
    let old_key = before.key().unwrap();
    let entry = save_effective(
        &mut storage,
        &dictionary,
        &old_key,
        0,
        "custom survives a later base match",
    );
    // Exact A metadata may introduce a built-in base match. Metadata is contextual,
    // and custom data must neither replace it nor be removed implicitly.
    let original = storage
        .chapter_analysis(imported.analysis_id)
        .unwrap()
        .unwrap();
    storage
        .store_chapter_analysis(
            imported.document.chapter.chapter_id,
            &ChapterAnalysis {
                profile: original.profile,
                mode: SplitMode::A,
                tokens: before
                    .tokens
                    .iter()
                    .map(|t| AnalyzedToken {
                        anchor: t.anchor,
                        surface: t.surface.clone(),
                        is_japanese: true,
                        reading: Some("analyzer reading".into()),
                        dictionary_form: Some(Surface::new("既存語").unwrap()),
                    })
                    .collect(),
            },
        )
        .unwrap();
    let frozen = snapshot(&mut storage, &imported).key().unwrap();
    let bytes = fs::read(database.path()).unwrap();
    rejected(&mut storage, &dictionary, &old_key, 0, stale);
    rejected(&mut storage, &dictionary, &frozen, 0, |e| {
        matches!(e, CustomDictionaryError::BuiltInMatch)
    });
    assert!(storage
        .load_effective_reader_word(&dictionary, &frozen, 0)
        .unwrap()
        .unwrap()
        .custom_entry
        .is_none());
    assert_eq!(
        crate::custom_dictionary::read_entry_in(&storage.connection, entry.surface().clone())
            .unwrap(),
        Some(entry)
    );
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn effective_custom_dictionary_rejects_reversed_replaced_forged_and_invalid_selections() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語だった English 。");
    let rule = split(&mut storage, &imported);
    let frozen = snapshot(&mut storage, &imported).key().unwrap();
    save_effective(&mut storage, &dictionary, &frozen, 0, "original");
    storage.reverse_segmentation_override(rule).unwrap();
    let bytes = fs::read(database.path()).unwrap();
    rejected(&mut storage, &dictionary, &frozen, 0, stale);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert_ne!(split(&mut storage, &imported), rule);
    let bytes = fs::read(database.path()).unwrap();
    rejected(&mut storage, &dictionary, &frozen, 0, stale);
    let current = snapshot(&mut storage, &imported).key().unwrap();
    for segment in [2, 3, u64::MAX] {
        rejected(&mut storage, &dictionary, &current, segment, stale);
    }
    let mut forged = current.clone();
    forged.segmentation.segments[0].boundary.surface = "既存語".into();
    rejected(&mut storage, &dictionary, &forged, 0, stale);
    let mut off_page = current.clone();
    off_page.page.range.end =
        TextOffset::at(&imported.document.chapter.text, "造語".len()).unwrap();
    rejected(&mut storage, &dictionary, &off_page, 1, stale);
    let mut invalid_page = current.clone();
    invalid_page.page.range.start = 1.try_into().unwrap();
    rejected(&mut storage, &dictionary, &invalid_page, 0, |e| {
        matches!(
            e,
            CustomDictionaryError::Selection(ReaderWordError::Storage(
                StorageError::InvalidAnalysis
            ))
        )
    });
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    rejected(&mut storage, &dictionary, &current, 0, stale);
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    assert_eq!(load_effective(&mut storage, &dictionary, &current, 0), None);
    assert!(storage
        .save_effective_reader_custom_entry(&dictionary, &current, 0, "late".into(), "かな".into())
        .unwrap()
        .is_none());
    assert!(!storage
        .delete_effective_reader_custom_entry(&dictionary, &current, 0)
        .unwrap());
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn effective_custom_dictionary_crossing_unicode_spans_share_full_identity_at_every_preset() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let tail = format!("{}終", "𠮷か\u{3099}。\0\n".repeat(600));
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new(format!("造語{tail}")).unwrap(),
            100,
            |chapter| {
                let mut analysis = fixture(chapter).unwrap();
                analysis.tokens.truncate(1);
                analysis.tokens[0].surface = chapter.text.clone();
                analysis.tokens[0].anchor.range.end =
                    TextOffset::at(&chapter.text, chapter.text.len()).unwrap();
                Ok::<_, Infallible>(analysis)
            },
        )
        .unwrap();
    let chapter = &imported.document.chapter;
    storage
        .save_segmentation_override(&SegmentationOverride {
            anchor: key(&imported).page,
            surface: Surface::new(&chapter.text).unwrap(),
            scope: OverrideScope::ThisOccurrence,
            parts: vec![Surface::new("造語").unwrap(), Surface::new(&tail).unwrap()],
        })
        .unwrap();
    let mut expected = None;
    for preset in PageLength::ALL {
        let pages = LogicalPages::new(std::slice::from_ref(chapter), preset).unwrap();
        assert!(pages.pages().len() > 1);
        for (index, page) in pages.pages().iter().enumerate() {
            let frozen = storage
                .load_effective_reader_tokens(imported.document.document_id, *page)
                .unwrap()
                .unwrap()
                .key()
                .unwrap();
            let entry = save_effective(&mut storage, &dictionary, &frozen, 1, "literal full span");
            assert_eq!(entry.surface().as_str(), tail);
            if let Some(expected) = &expected {
                assert_eq!(&entry, expected);
            }
            expected = Some(entry.clone());
            assert_eq!(
                load_effective(&mut storage, &dictionary, &frozen, 1),
                Some(entry)
            );
            if index != 0 {
                rejected(&mut storage, &dictionary, &frozen, 0, stale);
            }
        }
    }
    assert!(storage
        .vocabulary_entry(&Surface::new(tail).unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn effective_custom_dictionary_statement_and_commit_failures_roll_back_then_retry_exactly() {
    for operation in ["INSERT", "UPDATE", "DELETE"] {
        for deferred in [false, true] {
            let database = TestDatabase::new();
            let index = dictionary();
            let dictionary = Dictionary::open(index.path()).unwrap();
            let mut storage = Storage::open(database.path()).unwrap();
            let imported = import(&mut storage, "造語だった");
            split(&mut storage, &imported);
            let frozen = snapshot(&mut storage, &imported).key().unwrap();
            let previous = (operation != "INSERT")
                .then(|| save_effective(&mut storage, &dictionary, &frozen, 0, "original"));
            if deferred {
                storage.connection.execute_batch("CREATE TABLE deferred_custom (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED)").unwrap();
            }
            let trigger = if deferred { "AFTER" } else { "BEFORE" };
            let action = if deferred {
                "INSERT INTO deferred_custom VALUES (-1);"
            } else {
                "SELECT RAISE(ABORT, 'fixture');"
            };
            storage.connection.execute_batch(&format!("CREATE TRIGGER fail_custom {trigger} {operation} ON custom_dictionary_entries BEGIN {action} END;")).unwrap();
            let bytes = fs::read(database.path()).unwrap();
            let result = if operation == "DELETE" {
                storage
                    .delete_effective_reader_custom_entry(&dictionary, &frozen, 0)
                    .map(|_| ())
            } else {
                storage
                    .save_effective_reader_custom_entry(
                        &dictionary,
                        &frozen,
                        0,
                        "edited".into(),
                        "かな".into(),
                    )
                    .map(|_| ())
            };
            assert!(matches!(
                result,
                Err(CustomDictionaryError::Storage(StorageError::Sqlite(_)))
            ));
            assert_eq!(fs::read(database.path()).unwrap(), bytes);
            assert_eq!(
                load_effective(&mut storage, &dictionary, &frozen, 0),
                previous
            );
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_custom")
                .unwrap();
            if operation == "DELETE" {
                assert!(storage
                    .delete_effective_reader_custom_entry(&dictionary, &frozen, 0)
                    .unwrap());
            } else {
                assert_eq!(
                    save_effective(&mut storage, &dictionary, &frozen, 0, "edited")
                        .english_meaning(),
                    "edited"
                );
            }
        }
    }
}

#[test]
fn effective_custom_dictionary_noop_retries_invalid_fields_and_corruption_preserve_saved_data() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語だった");
    split(&mut storage, &imported);
    let frozen = snapshot(&mut storage, &imported).key().unwrap();
    save_effective(&mut storage, &dictionary, &frozen, 0, "original");
    storage.connection.execute_batch("CREATE TRIGGER fail_custom BEFORE UPDATE ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT, 'must be no-op'); END;").unwrap();
    let bytes = fs::read(database.path()).unwrap();
    save_effective(&mut storage, &dictionary, &frozen, 0, "original");
    for (meaning, reading) in [("", "かな"), ("meaning", "　\n")] {
        assert!(matches!(
            storage.save_effective_reader_custom_entry(
                &dictionary,
                &frozen,
                0,
                meaning.into(),
                reading.into()
            ),
            Err(CustomDictionaryError::InvalidEntry)
        ));
    }
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_custom; PRAGMA ignore_check_constraints = ON")
        .unwrap();
    for sql in [
        "UPDATE custom_dictionary_entries SET semantics_version = 2",
        "UPDATE custom_dictionary_entries SET semantics_version = 1, english_meaning = '　'",
        "UPDATE custom_dictionary_entries SET english_meaning = 'original', reading = ''",
    ] {
        storage.connection.execute_batch(sql).unwrap();
        let bytes = fs::read(database.path()).unwrap();
        rejected(&mut storage, &dictionary, &frozen, 0, |e| {
            matches!(e, CustomDictionaryError::InvalidEntry)
        });
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}

#[test]
fn effective_custom_dictionary_dictionary_failure_never_allows_custom_mutation_and_retry_recovers()
{
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語だった");
    split(&mut storage, &imported);
    let frozen = snapshot(&mut storage, &imported).key().unwrap();
    let entry = save_effective(&mut storage, &dictionary, &frozen, 0, "original");
    let writer = Connection::open(index.path()).unwrap();
    writer
        .execute_batch(
            "UPDATE entries SET payload = '{}'; INSERT INTO forms VALUES ('造語', 'JMdict', 1)",
        )
        .unwrap();
    let bytes = fs::read(database.path()).unwrap();
    let index_bytes = fs::read(index.path()).unwrap();
    rejected(&mut storage, &dictionary, &frozen, 0, |e| {
        matches!(
            e,
            CustomDictionaryError::Selection(ReaderWordError::Dictionary(_))
        )
    });
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert_eq!(fs::read(index.path()).unwrap(), index_bytes);
    writer
        .execute("DELETE FROM forms WHERE surface = '造語'", [])
        .unwrap();
    assert_eq!(
        load_effective(&mut storage, &dictionary, &frozen, 0),
        Some(entry)
    );
    save_effective(&mut storage, &dictionary, &frozen, 0, "explicit retry");
}
