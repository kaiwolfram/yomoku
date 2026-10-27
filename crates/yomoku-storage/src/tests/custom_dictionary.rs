use std::{
    convert::Infallible,
    fs,
    sync::{Arc, Barrier},
};

use rusqlite::Connection;
use serde_json::json;
use yomoku_core::{
    dictionary::CustomDictionaryEntry,
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    pagination::{LogicalPages, PageLength},
    reader::{ReaderAnalysis, ReaderTokenKey},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{history, imports::fixture, TestDatabase};
use crate::{
    custom_dictionary::CustomDictionaryError, dictionary::Dictionary, migrate,
    reader_words::ReaderWordError, AnalyzedPaste, Storage, StorageError, MIGRATIONS,
};

fn dictionary() -> TestDatabase {
    let file = TestDatabase::new();
    let connection = Connection::open(file.path()).unwrap();
    connection.execute_batch("PRAGMA application_id = 0x594f4449; PRAGMA user_version = 1;
        CREATE TABLE entries (source TEXT, entry_id INTEGER, payload TEXT, PRIMARY KEY(source, entry_id)) WITHOUT ROWID;
        CREATE TABLE forms (surface TEXT COLLATE BINARY, source TEXT, entry_id INTEGER, PRIMARY KEY(surface, source, entry_id)) WITHOUT ROWID;").unwrap();
    for (source, surface, payload) in [
        (
            "JMdict",
            "既存語",
            json!({"kanji": [{"text": "既存語"}], "readings": [{"text": "きそんご"}],
            "senses": [{"glosses": [{"text": "existing word", "lang": "eng"}]}]}),
        ),
        (
            "JMnedict",
            "人名",
            json!({"kanji": [{"text": "人名"}], "readings": [{"text": "じんめい"}],
            "names": [{"name_type": ["person"], "translations": [{"text": "Name", "lang": "eng"}]}]}),
        ),
    ] {
        connection
            .execute(
                "INSERT INTO entries VALUES (?1, 1, ?2)",
                (source, payload.to_string()),
            )
            .unwrap();
        connection
            .execute("INSERT INTO forms VALUES (?1, ?2, 1)", (surface, source))
            .unwrap();
    }
    file
}

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, |chapter| {
            let mut analysis = fixture(chapter)?;
            for token in &mut analysis.tokens {
                if token.surface == "既存語だった" {
                    token.dictionary_form = Some(Surface::new("既存語").unwrap());
                }
            }
            Ok::<_, Infallible>(analysis)
        })
        .unwrap()
}

fn key(imported: &AnalyzedPaste) -> ReaderTokenKey {
    let chapter = &imported.document.chapter;
    ReaderTokenKey {
        document_id: imported.document.document_id,
        page: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: 0.try_into().unwrap(),
                end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
            },
        },
        analysis: ReaderAnalysis::Available(imported.analysis_id),
    }
}

fn save(
    storage: &mut Storage,
    dictionary: &Dictionary,
    key: ReaderTokenKey,
    meaning: &str,
    reading: &str,
) -> CustomDictionaryEntry {
    storage
        .save_reader_custom_entry(dictionary, key, 0, meaning.into(), reading.into())
        .unwrap()
        .unwrap()
}

fn load(
    storage: &mut Storage,
    dictionary: &Dictionary,
    key: ReaderTokenKey,
) -> Option<CustomDictionaryEntry> {
    storage
        .load_reader_custom_entry(dictionary, key, 0)
        .unwrap()
}

#[test]
fn custom_dictionary_create_edit_read_delete_reopen_are_global_without_reader_side_effects() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let index_before = fs::read(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "造語 造語");
    let second = import(&mut storage, "造語 別語");
    let tokens = storage
        .load_reader_tokens(key(&first).document_id, key(&first).page)
        .unwrap();
    let snapshot = storage
        .load_document_reader_snapshot(key(&first).document_id)
        .unwrap();
    let details = storage
        .load_reader_word(&dictionary, key(&first), 0)
        .unwrap();
    assert_eq!(load(&mut storage, &dictionary, key(&first)), None);
    let original = save(&mut storage, &dictionary, key(&first), "new word", "ぞうご");
    assert_eq!(
        load(&mut storage, &dictionary, key(&second)),
        Some(original)
    );
    let edited = save(
        &mut storage,
        &dictionary,
        key(&second),
        " literal\nmeaning 📖\0 ",
        " か\u{3099}𠮷\0 ",
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        load(&mut storage, &dictionary, key(&first)),
        Some(edited.clone())
    );
    assert_eq!(
        storage
            .load_reader_tokens(key(&first).document_id, key(&first).page)
            .unwrap(),
        tokens
    );
    assert_eq!(
        storage
            .load_document_reader_snapshot(key(&first).document_id)
            .unwrap(),
        snapshot
    );
    assert_eq!(
        storage
            .load_reader_word(&dictionary, key(&first), 0)
            .unwrap(),
        details
    );
    let counts: (i64, i64, i64) = storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM vocabulary_status_events), (SELECT COUNT(*) FROM reading_occurrences), (SELECT COUNT(*) FROM reading_completions)", [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(counts, (2, 0, 0));
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert_eq!(load(&mut storage, &dictionary, key(&second)), Some(edited));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    assert!(storage
        .delete_reader_custom_entry(&dictionary, key(&second), 0)
        .unwrap());
    assert!(!storage
        .delete_reader_custom_entry(&dictionary, key(&first), 0)
        .unwrap());
    drop(storage);
    assert_eq!(
        load(
            &mut Storage::open(database.path()).unwrap(),
            &dictionary,
            key(&first)
        ),
        None
    );
    assert_eq!(fs::read(index.path()).unwrap(), index_before);
}

#[test]
fn custom_dictionary_exact_spelling_never_normalizes_or_follows_custom_base_entries() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "が か\u{3099} ガ 𠮷 造語\0 造語 'OR1=1--");
    let tokens = storage
        .load_reader_tokens(key(&imported).document_id, key(&imported).page)
        .unwrap()
        .unwrap()
        .tokens;
    for token in &tokens {
        storage
            .save_reader_custom_entry(
                &dictionary,
                key(&imported),
                token.token_index,
                format!("meaning {}", token.token_index),
                "reading".into(),
            )
            .unwrap();
    }
    for token in tokens {
        let found = storage
            .load_reader_custom_entry(&dictionary, key(&imported), token.token_index)
            .unwrap()
            .unwrap();
        assert_eq!(found.surface().as_str(), token.surface);
        assert_eq!(
            found.english_meaning(),
            format!("meaning {}", token.token_index)
        );
    }
    let base = import(&mut storage, "造語基底");
    save(&mut storage, &dictionary, key(&base), "base only", "きてい");
    let inflected = storage
        .create_analyzed_paste(
            &PasteText::new("造語基底だった".into()).unwrap(),
            101,
            |chapter| {
                let mut analysis = fixture(chapter)?;
                analysis.tokens[0].dictionary_form = Some(Surface::new("造語基底").unwrap());
                Ok::<_, Infallible>(analysis)
            },
        )
        .unwrap();
    assert_eq!(load(&mut storage, &dictionary, key(&inflected)), None);
}

#[test]
fn custom_dictionary_built_in_words_names_and_base_matches_reject_every_operation() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    for text in ["既存語", "人名", "既存語だった"] {
        let imported = import(&mut storage, text);
        let before = fs::read(database.path()).unwrap();
        let index_before = fs::read(index.path()).unwrap();
        assert!(matches!(
            storage.save_reader_custom_entry(
                &dictionary,
                key(&imported),
                0,
                "override".into(),
                "かな".into()
            ),
            Err(CustomDictionaryError::BuiltInMatch)
        ));
        assert!(matches!(
            storage.load_reader_custom_entry(&dictionary, key(&imported), 0),
            Err(CustomDictionaryError::BuiltInMatch)
        ));
        assert!(matches!(
            storage.delete_reader_custom_entry(&dictionary, key(&imported), 0),
            Err(CustomDictionaryError::BuiltInMatch)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert_eq!(fs::read(index.path()).unwrap(), index_before);
    }
}

#[test]
fn custom_dictionary_validates_selection_and_does_not_recreate_deleted_entries_or_documents() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語 English");
    let other = import(&mut storage, "造語");
    let mut stale = key(&imported);
    stale.analysis = ReaderAnalysis::Missing;
    let mut foreign = key(&other);
    foreign.document_id = imported.document.document_id;
    for (key, token) in [
        (stale, 0),
        (foreign, 0),
        (key(&imported), 1),
        (key(&imported), 999),
    ] {
        let before = fs::read(database.path()).unwrap();
        assert!(storage
            .load_reader_custom_entry(&dictionary, key, token)
            .is_err());
        assert!(storage
            .save_reader_custom_entry(&dictionary, key, token, "meaning".into(), "かな".into())
            .is_err());
        assert!(storage
            .delete_reader_custom_entry(&dictionary, key, token)
            .is_err());
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    let entry = save(
        &mut storage,
        &dictionary,
        key(&imported),
        "new word",
        "ぞうご",
    );
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        let id = storage
            .vocabulary_entry(entry.surface())
            .unwrap()
            .unwrap()
            .id;
        storage.set_vocabulary_status(id, status, 102).unwrap();
        assert_eq!(
            load(&mut storage, &dictionary, key(&other)),
            Some(entry.clone())
        );
    }
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '造語'", [])
        .unwrap();
    assert_eq!(
        load(&mut storage, &dictionary, key(&imported)),
        Some(entry.clone())
    );
    save(
        &mut storage,
        &dictionary,
        key(&imported),
        "edited",
        "ぞうご",
    );
    assert!(storage.vocabulary_entry(entry.surface()).unwrap().is_none());
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert_eq!(load(&mut storage, &dictionary, key(&imported)), None);
    assert_eq!(
        storage
            .save_reader_custom_entry(&dictionary, key(&imported), 0, "gone".into(), "かな".into())
            .unwrap(),
        None
    );
    assert!(!storage
        .delete_reader_custom_entry(&dictionary, key(&imported), 0)
        .unwrap());
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(
        load(&mut storage, &dictionary, key(&other))
            .unwrap()
            .english_meaning(),
        "edited"
    );
}

#[test]
fn custom_dictionary_crossing_tokens_keep_one_global_entry_under_all_page_presets() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = "𠮷か\u{3099}。".repeat(1000);
    let imported = import(&mut storage, &text);
    let entry = save(
        &mut storage,
        &dictionary,
        key(&imported),
        "complete long token",
        "かな",
    );
    for preset in PageLength::ALL {
        let chapter = &imported.document.chapter;
        let pages = LogicalPages::new(std::slice::from_ref(chapter), preset).unwrap();
        assert!(pages.pages().len() > 1);
        for page in pages.pages() {
            let key = ReaderTokenKey {
                page: *page,
                ..key(&imported)
            };
            assert_eq!(load(&mut storage, &dictionary, key), Some(entry.clone()));
            assert_eq!(
                save(
                    &mut storage,
                    &dictionary,
                    key,
                    "complete long token",
                    "かな"
                ),
                entry
            );
        }
    }
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM custom_dictionary_entries", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn custom_dictionary_statement_and_commit_failures_roll_back_create_edit_delete_and_retry() {
    for operation in ["INSERT", "UPDATE", "DELETE"] {
        for deferred in [false, true] {
            let database = TestDatabase::new();
            let index = dictionary();
            let dictionary = Dictionary::open(index.path()).unwrap();
            let mut storage = Storage::open(database.path()).unwrap();
            let imported = import(&mut storage, "造語");
            let key = key(&imported);
            let previous = if operation == "INSERT" {
                None
            } else {
                Some(save(&mut storage, &dictionary, key, "original", "かな"))
            };
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
            let before = fs::read(database.path()).unwrap();
            if operation == "DELETE" {
                assert!(matches!(
                    storage.delete_reader_custom_entry(&dictionary, key, 0),
                    Err(CustomDictionaryError::Storage(StorageError::Sqlite(_)))
                ));
            } else {
                assert!(matches!(
                    storage.save_reader_custom_entry(
                        &dictionary,
                        key,
                        0,
                        "edited".into(),
                        "よみ".into()
                    ),
                    Err(CustomDictionaryError::Storage(StorageError::Sqlite(_)))
                ));
            }
            assert_eq!(fs::read(database.path()).unwrap(), before);
            assert_eq!(load(&mut storage, &dictionary, key), previous);
            storage
                .connection
                .execute_batch("DROP TRIGGER fail_custom")
                .unwrap();
            if operation == "DELETE" {
                assert!(storage
                    .delete_reader_custom_entry(&dictionary, key, 0)
                    .unwrap());
            } else {
                assert_eq!(
                    save(&mut storage, &dictionary, key, "edited", "よみ").english_meaning(),
                    "edited"
                );
            }
        }
    }
}

#[test]
fn custom_dictionary_invalid_input_corruption_and_identical_retries_never_repair_or_write() {
    let database = TestDatabase::new();
    let index = dictionary();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語");
    let key = key(&imported);
    save(&mut storage, &dictionary, key, "original", "かな");
    for (meaning, reading) in [("", "かな"), ("meaning", "　\n"), ("\t", "かな")] {
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.save_reader_custom_entry(&dictionary, key, 0, meaning.into(), reading.into()),
            Err(CustomDictionaryError::InvalidEntry)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    storage.connection.execute_batch("CREATE TRIGGER fail_custom BEFORE UPDATE ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT, 'must be no-op'); END;").unwrap();
    let before = fs::read(database.path()).unwrap();
    save(&mut storage, &dictionary, key, "original", "かな");
    assert_eq!(fs::read(database.path()).unwrap(), before);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_custom; PRAGMA ignore_check_constraints = ON;")
        .unwrap();
    for sql in [
        "UPDATE custom_dictionary_entries SET semantics_version = 2",
        "UPDATE custom_dictionary_entries SET semantics_version = 1, english_meaning = '　'",
        "UPDATE custom_dictionary_entries SET english_meaning = 'original', reading = ''",
    ] {
        storage.connection.execute_batch(sql).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            storage.load_reader_custom_entry(&dictionary, key, 0),
            Err(CustomDictionaryError::InvalidEntry)
        ));
        assert!(matches!(
            storage.save_reader_custom_entry(&dictionary, key, 0, "repair".into(), "かな".into()),
            Err(CustomDictionaryError::InvalidEntry)
        ));
        assert!(matches!(
            storage.delete_reader_custom_entry(&dictionary, key, 0),
            Err(CustomDictionaryError::InvalidEntry)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn custom_dictionary_failed_builtin_lookup_is_never_treated_as_a_missing_match() {
    let database = TestDatabase::new();
    let index = dictionary();
    let writer = Connection::open(index.path()).unwrap();
    writer
        .execute_batch(
            "UPDATE entries SET payload = '{}'; INSERT INTO forms VALUES ('造語', 'JMdict', 1)",
        )
        .unwrap();
    let dictionary = Dictionary::open(index.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語");
    let before = fs::read(database.path()).unwrap();
    let index_before = fs::read(index.path()).unwrap();
    assert!(matches!(
        storage.save_reader_custom_entry(
            &dictionary,
            key(&imported),
            0,
            "fallback".into(),
            "かな".into()
        ),
        Err(CustomDictionaryError::Selection(
            ReaderWordError::Dictionary(_)
        ))
    ));
    assert!(matches!(
        storage.load_reader_custom_entry(&dictionary, key(&imported), 0),
        Err(CustomDictionaryError::Selection(
            ReaderWordError::Dictionary(_)
        ))
    ));
    assert!(matches!(
        storage.delete_reader_custom_entry(&dictionary, key(&imported), 0),
        Err(CustomDictionaryError::Selection(
            ReaderWordError::Dictionary(_)
        ))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(fs::read(index.path()).unwrap(), index_before);
}

#[test]
fn custom_dictionary_migration_preserves_v16_reader_state_and_rolls_back_failed_upgrade() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..16]).unwrap();
    let mut old = Storage { connection };
    let imported = import(&mut old, "造語 𠮷");
    old.save_page_length(PageLength::Characters750).unwrap();
    let snapshot = old
        .load_document_reader_snapshot(key(&imported).document_id)
        .unwrap();
    old.connection
        .execute_batch("CREATE TABLE custom_dictionary_entries (fixture TEXT)")
        .unwrap();
    drop(old);
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(history(&database.path()).len(), 16);
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch("DROP TABLE custom_dictionary_entries")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 17);
    assert_eq!(
        storage
            .load_document_reader_snapshot(key(&imported).document_id)
            .unwrap(),
        snapshot
    );
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM custom_dictionary_entries", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn custom_dictionary_concurrent_saves_keep_one_complete_entry() {
    let database = TestDatabase::new();
    let index = dictionary();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "造語");
    let barrier = Arc::new(Barrier::new(2));
    std::thread::scope(|scope| {
        for (meaning, reading) in [("first", "いち"), ("second", "に")] {
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            let index = index.path();
            let key = key(&imported);
            scope.spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                let dictionary = Dictionary::open(index).unwrap();
                barrier.wait();
                save(&mut storage, &dictionary, key, meaning, reading);
            });
        }
    });
    let dictionary = Dictionary::open(index.path()).unwrap();
    let entry = load(&mut storage, &dictionary, key(&imported)).unwrap();
    assert!(matches!(
        (entry.english_meaning(), entry.reading()),
        ("first", "いち") | ("second", "に")
    ));
    let count: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM custom_dictionary_entries", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}
