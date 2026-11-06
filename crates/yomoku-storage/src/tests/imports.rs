use std::convert::Infallible;

use yomoku_core::{
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    vocabulary::{Surface, VocabularyStatus},
};

use super::*;

pub(super) fn fixture(chapter: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    // Synthetic classification only; real engine composition is tested separately.
    let mut cursor = 0;
    let tokens = chapter
        .text
        .split_whitespace()
        .map(|surface| {
            let start = cursor + chapter.text[cursor..].find(surface).unwrap();
            cursor = start + surface.len();
            AnalyzedToken {
                anchor: TextAnchor {
                    source_id: chapter.source_id,
                    chapter_id: chapter.chapter_id,
                    range: TextRange {
                        start: TextOffset::at(&chapter.text, start).unwrap(),
                        end: TextOffset::at(&chapter.text, cursor).unwrap(),
                    },
                },
                surface: surface.into(),
                is_japanese: !matches!(surface, "English" | "。" | "📖" | "\0"),
                reading: None,
                dictionary_form: None,
            }
        })
        .collect();
    Ok(ChapterAnalysis {
        profile: AnalysisProfile {
            engine_revision: "import-fixture@1".into(),
            dictionary_revision: "import-fixture-full@1".into(),
        },
        mode: SplitMode::C,
        tokens,
    })
}

fn count(storage: &Storage, table: &str) -> i64 {
    storage
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn totals(storage: &Storage) -> Vec<i64> {
    [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_positions",
        "reading_completions",
        "completion_commands",
    ]
    .map(|table| count(storage, table))
    .to_vec()
}

#[test]
fn analyzed_import_registers_unique_japanese_surfaces_and_reopens_exact_snapshot() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let observer = Storage::open(database.path()).unwrap();
    let text = PasteText::new("猫 猫\r\n𠮷 が か\u{3099} English 。 📖 \0".into()).unwrap();
    let imported = storage
        .analyzed_fixture(&text, 123, |chapter| {
            // Another connection cannot see any partial document or registration.
            assert_eq!(totals(&observer), vec![0; 11]);
            assert_eq!(chapter.text, text.as_str());
            fixture(chapter)
        })
        .unwrap();
    assert_eq!(imported.registered_surfaces, 4);
    assert!(imported.document.reading_position.is_none());
    assert_eq!(count(&storage, "vocabulary_status_events"), 4);
    assert_eq!(count(&storage, "reading_completions"), 0);
    assert_eq!(count(&storage, "reading_positions"), 0);
    drop(observer);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.load_paste(imported.document.document_id).unwrap(),
        Some(imported.document.clone())
    );
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(fixture(&imported.document.chapter).unwrap())
    );
    for word in ["猫", "𠮷", "が", "か\u{3099}"] {
        let entry = storage
            .vocabulary_entry(&Surface::new(word).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(entry.status, VocabularyStatus::New);
        assert_eq!(entry.registered_at_unix_ms, 123);
    }
    for other in ["English", "。", "📖", "\0"] {
        assert!(storage
            .vocabulary_entry(&Surface::new(other).unwrap())
            .unwrap()
            .is_none());
    }
}

#[test]
fn analyzed_import_preserves_every_existing_status_date_id_and_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut entries = Vec::new();
    for (word, status) in [
        ("猫", VocabularyStatus::New),
        ("犬", VocabularyStatus::Learning),
        ("鳥", VocabularyStatus::Familiar),
        ("魚", VocabularyStatus::Known),
        ("馬", VocabularyStatus::Ignored),
    ] {
        let surface = Surface::new(word).unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&surface), 10)
            .unwrap();
        let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
        entries.push(
            storage
                .set_vocabulary_status(entry.id, status, 20)
                .unwrap()
                .unwrap(),
        );
    }
    let events = count(&storage, "vocabulary_status_events");
    let imported = storage
        .analyzed_fixture(
            &PasteText::new("猫 犬 鳥 魚 馬 𠮷 猫".into()).unwrap(),
            30,
            fixture,
        )
        .unwrap();
    assert_eq!(imported.registered_surfaces, 1);
    for entry in entries {
        assert_eq!(
            storage.vocabulary_entry(&entry.surface).unwrap(),
            Some(entry)
        );
    }
    assert_eq!(count(&storage, "vocabulary_status_events"), events + 1);
}

#[test]
fn analyzer_errors_invalid_output_and_invalid_dates_leave_no_partial_import() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = PasteText::new("猫 𠮷".into()).unwrap();
    let before = totals(&storage);
    let error = storage.analyzed_fixture(&text, 1, |_| {
        Err::<ChapterAnalysis, _>("analyzer unavailable")
    });
    assert!(matches!(
        error,
        Err(PasteImportError::Analysis("analyzer unavailable"))
    ));
    for variant in 0..6 {
        let result = storage.analyzed_fixture(&text, 1, |chapter| {
            let mut analysis = fixture(chapter).unwrap();
            match variant {
                0 => analysis.mode = SplitMode::A,
                1 => analysis.mode = SplitMode::B,
                2 => analysis.tokens[0].anchor.source_id = i64::MAX.try_into().unwrap(),
                3 => analysis.tokens[1].anchor.range.end = 1.try_into().unwrap(),
                4 => analysis.tokens[0].surface = "別".into(),
                _ => analysis.tokens.clear(),
            }
            Ok::<_, Infallible>(analysis)
        });
        assert!(matches!(
            result,
            Err(PasteImportError::Storage(StorageError::InvalidAnalysis))
        ));
        assert_eq!(totals(&storage), before);
    }
    let result = storage.analyzed_fixture(&text, -1, |_| -> Result<ChapterAnalysis, Infallible> {
        panic!("invalid date must precede analysis")
    });
    assert!(matches!(
        result,
        Err(PasteImportError::Storage(StorageError::InvalidVocabulary))
    ));
    assert_eq!(totals(&storage), before);
    assert_eq!(
        storage
            .analyzed_fixture(&text, 1, fixture)
            .unwrap()
            .registered_surfaces,
        2
    );
}

#[test]
fn every_import_write_and_final_commit_failure_roll_back_and_retry() {
    for table in [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "commit",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let existing = Surface::new("猫").unwrap();
        storage.register_vocabulary(&[existing], 10).unwrap();
        let before = totals(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
                CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TEMP TRIGGER fail_import AFTER INSERT ON vocabulary_status_events
                BEGIN INSERT INTO deferred_child VALUES (99); END;").unwrap();
        } else {
            // Fail after earlier rows (and, for tokens/vocabulary, earlier values) were inserted.
            storage.connection.execute_batch(&format!(
                "CREATE TEMP TRIGGER fail_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
            )).unwrap();
        }
        let text = PasteText::new("猫 犬 𠮷".into()).unwrap();
        assert!(
            matches!(
                storage.analyzed_fixture(&text, 20, fixture),
                Err(PasteImportError::Storage(StorageError::Sqlite(_)))
            ),
            "{table}"
        );
        assert_eq!(totals(&storage), before, "{table}");
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_import;")
            .unwrap();
        let imported = storage.analyzed_fixture(&text, 20, fixture).unwrap();
        assert_eq!(imported.registered_surfaces, 2);
        drop(storage);
        let mut reopened = Storage::open(database.path()).unwrap();
        assert_eq!(
            reopened.load_paste(imported.document.document_id).unwrap(),
            Some(imported.document)
        );
        assert_eq!(count(&reopened, "vocabulary_status_events"), 3);
        assert_eq!(count(&reopened, "reading_completions"), 0);
    }
}

#[test]
fn concurrent_imports_share_global_surfaces_with_independent_anchors() {
    let database = TestDatabase::new();
    Storage::open(database.path()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                let text = if index == 0 {
                    "猫 𠮷"
                } else {
                    "𠮷 猫 猫"
                };
                storage
                    .analyzed_fixture(&PasteText::new(text.into()).unwrap(), 50, fixture)
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let imports: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        imports
            .iter()
            .map(|import| import.registered_surfaces)
            .sum::<usize>(),
        2
    );
    assert_ne!(
        imports[0].document.document_id,
        imports[1].document.document_id
    );
    assert_ne!(
        imports[0].document.chapter.source_id,
        imports[1].document.chapter.source_id
    );
    let mut storage = Storage::open(database.path()).unwrap();
    for import in imports {
        assert_eq!(
            storage.chapter_analysis(import.analysis_id).unwrap(),
            Some(fixture(&import.document.chapter).unwrap())
        );
    }
    assert_eq!(count(&storage, "vocabulary_status_events"), 2);
}

#[test]
fn non_japanese_import_stores_analysis_without_vocabulary_or_encounters() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .analyzed_fixture(
            &PasteText::new("English 。 📖 \0".into()).unwrap(),
            0,
            fixture,
        )
        .unwrap();
    assert_eq!(imported.registered_surfaces, 0);
    assert_eq!(count(&storage, "analyzed_tokens"), 4);
    assert_eq!(count(&storage, "vocabulary"), 0);
    assert_eq!(count(&storage, "vocabulary_status_events"), 0);
    assert_eq!(count(&storage, "reading_completions"), 0);
}
