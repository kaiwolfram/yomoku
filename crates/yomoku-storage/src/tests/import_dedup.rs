use std::{convert::Infallible, sync::atomic::AtomicUsize};

use yomoku_core::{
    analysis::ChapterAnalysis,
    completion::{CompletionOutcome, PageCompletion},
    document::{PasteText, TextOffset},
    pagination::{LogicalPages, ReadingPosition},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn text(value: &str) -> PasteText {
    PasteText::new(value.into()).unwrap()
}

fn created(outcome: PasteImportOutcome) -> AnalyzedPaste {
    match outcome {
        PasteImportOutcome::Created(imported) => imported,
        PasteImportOutcome::Existing(_) => panic!("expected a new import"),
    }
}

fn existing(outcome: PasteImportOutcome) -> yomoku_core::document::PastedDocument {
    match outcome {
        PasteImportOutcome::Existing(document) => document,
        PasteImportOutcome::Created(_) => panic!("expected an existing document"),
    }
}

fn must_not_analyze(_: &yomoku_core::document::ChapterText) -> Result<ChapterAnalysis, Infallible> {
    panic!("duplicate import must skip analysis")
}

fn count(storage: &Storage, table: &str) -> i64 {
    storage
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn state(storage: &Storage) -> Vec<i64> {
    [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "paste_content_hashes",
        "reading_positions",
        "reading_completions",
        "completion_commands",
    ]
    .map(|table| count(storage, table))
    .to_vec()
}

#[test]
fn duplicate_reopens_existing_title_position_status_and_analysis_without_writes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = text("猫 犬 猫\r\n𠮷 📖 \0");
    let imported = created(storage.import_analyzed_paste(&input, 10, fixture).unwrap());
    assert_eq!(imported.registered_surfaces, 3);
    let analysis = storage.chapter_analysis(imported.analysis_id).unwrap();
    let cat = Surface::new("猫").unwrap();
    let entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 20)
        .unwrap();
    storage
        .connection
        .execute("UPDATE documents SET title = '改題'", [])
        .unwrap();
    let position = ReadingPosition {
        source_id: imported.document.chapter.source_id,
        chapter_id: imported.document.chapter.chapter_id,
        offset: TextOffset::at(input.as_str(), "猫 犬".len()).unwrap(),
    };
    let expected = storage
        .save_reading_position(imported.document.document_id, position)
        .unwrap()
        .unwrap();
    let before = state(&storage);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let changes = storage.connection.total_changes();
    let duplicate = existing(
        storage
            .import_analyzed_paste(&input, 30, must_not_analyze)
            .unwrap(),
    );
    assert_eq!(duplicate, expected);
    assert_eq!(state(&storage), before);
    assert_eq!(storage.connection.total_changes(), changes);
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        analysis
    );
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), known);
    // Simulate future confirmed vocabulary deletion: duplicate import is not a rescan.
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '犬'", [])
        .unwrap();
    existing(
        storage
            .import_analyzed_paste(&input, 40, must_not_analyze)
            .unwrap(),
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("犬").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn identity_uses_sha256_of_exact_utf8_including_nul_line_endings_and_unicode() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let values = [
        "abc",
        "abc\0",
        "abc\0x",
        "abc\0y",
        "題\nが",
        "題\nか\u{3099}",
        "題\r\nが",
        "題\nが ",
        "題\n𠮷📖",
    ];
    let mut ids = Vec::new();
    for value in values {
        let input = text(value);
        let imported = created(storage.import_analyzed_paste(&input, 0, fixture).unwrap());
        let duplicate = existing(
            storage
                .import_analyzed_paste(&input, 1, must_not_analyze)
                .unwrap(),
        );
        assert_eq!(duplicate, imported.document);
        assert!(!ids.contains(&duplicate.document_id));
        ids.push(duplicate.document_id);
    }
    let digest: String = storage
        .connection
        .query_row(
            "SELECT hex(sha256) FROM paste_content_hashes WHERE document_id = ?1",
            [i64::from(ids[0])],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        digest,
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
    );
    assert_eq!(count(&storage, "documents"), values.len() as i64);
}

#[test]
fn upgrades_legacy_duplicates_lazily_and_references_oldest_live_document() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..7]).unwrap();
    let mut old = Storage { connection };
    let input = text("猫 𠮷");
    let first = old.create_paste(&input).unwrap();
    let second = old.create_paste(&input).unwrap();
    let other = old.create_paste(&text("犬")).unwrap();
    let before = old.list_documents().unwrap();
    drop(old);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.list_documents().unwrap(), before);
    assert_eq!(count(&storage, "paste_content_hashes"), 0);
    assert_eq!(
        existing(
            storage
                .import_analyzed_paste(&input, 10, must_not_analyze)
                .unwrap()
        ),
        first
    );
    assert_eq!(count(&storage, "paste_content_hashes"), 3);
    assert_eq!(storage.list_documents().unwrap(), before);
    assert_eq!(
        storage.load_paste(second.document_id).unwrap(),
        Some(second)
    );
    assert_eq!(storage.load_paste(other.document_id).unwrap(), Some(other));
    assert_eq!(count(&storage, "vocabulary"), 0);
    assert_eq!(count(&storage, "chapter_analyses"), 0);
    // Raw creation remains available for repository foundations; its missing
    // hash is discovered even after the first lazy indexing pass.
    let later = storage.create_paste(&text("鳥")).unwrap();
    assert_eq!(
        existing(
            storage
                .import_analyzed_paste(&text("鳥"), 20, must_not_analyze)
                .unwrap()
        ),
        later
    );
}

#[test]
fn concurrent_identical_imports_create_one_document_and_analyze_once() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    let barrier = Arc::new(Barrier::new(4));
    let analyses = AtomicUsize::new(0);
    let outcomes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                let analyses = &analyses;
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .import_analyzed_paste(&text("猫 猫 𠮷"), 10, |chapter| {
                            analyses.fetch_add(1, Ordering::Relaxed);
                            fixture(chapter)
                        })
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(analyses.load(Ordering::Relaxed), 1);
    let mut new = 0;
    let documents: Vec<_> = outcomes
        .into_iter()
        .map(|outcome| match outcome {
            PasteImportOutcome::Created(imported) => {
                new += 1;
                imported.document
            }
            PasteImportOutcome::Existing(document) => document,
        })
        .collect();
    assert_eq!(new, 1);
    assert!(documents.iter().all(|document| *document == documents[0]));
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(count(&storage, "documents"), 1);
    assert_eq!(count(&storage, "paste_content_hashes"), 1);
    assert_eq!(count(&storage, "vocabulary_status_events"), 2);
}

#[test]
fn failures_at_every_stage_roll_back_new_content_hashes_and_legacy_indexing() {
    for table in [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "paste_content_hashes",
        "commit",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let saved = storage.create_paste(&text("既存")).unwrap();
        let before = state(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY);
                CREATE TEMP TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TEMP TRIGGER fail_import AFTER INSERT ON paste_content_hashes
                BEGIN INSERT INTO child VALUES (99); END;").unwrap();
        } else {
            // Hash failure occurs on the new document after its analysis/vocab
            // writes, and after the legacy hash has been written provisionally.
            let condition = if table == "paste_content_hashes" {
                "WHEN NEW.document_id > 1"
            } else {
                ""
            };
            storage
                .connection
                .execute_batch(&format!(
                    "CREATE TEMP TRIGGER fail_import AFTER INSERT ON {table} {condition}
                 BEGIN SELECT RAISE(ABORT, 'fixture'); END;"
                ))
                .unwrap();
        }
        assert!(
            matches!(
                storage.import_analyzed_paste(&text("猫 犬"), 10, fixture),
                Err(PasteImportError::Storage(StorageError::Sqlite(_)))
            ),
            "{table}"
        );
        assert_eq!(state(&storage), before, "{table}");
        assert_eq!(storage.load_paste(saved.document_id).unwrap(), Some(saved));
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_import")
            .unwrap();
        let imported = created(
            storage
                .import_analyzed_paste(&text("猫 犬"), 10, fixture)
                .unwrap(),
        );
        assert_eq!(imported.registered_surfaces, 2);
        assert_eq!(count(&storage, "paste_content_hashes"), 2);
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(
            existing(
                storage
                    .import_analyzed_paste(&text("猫 犬"), 20, must_not_analyze)
                    .unwrap()
            ),
            imported.document
        );
    }
}

#[test]
fn analyzer_validation_and_indexing_errors_leave_previous_imports_intact() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let saved = created(
        storage
            .import_analyzed_paste(&text("猫"), 10, fixture)
            .unwrap(),
    );
    storage.create_paste(&text("既存")).unwrap();
    let before = state(&storage);
    let error = storage.import_analyzed_paste(&text("犬"), 20, |_| Err("typed failure"));
    assert!(matches!(
        error,
        Err(PasteImportError::Analysis("typed failure"))
    ));
    assert_eq!(state(&storage), before);
    let error = storage.import_analyzed_paste(&text("犬"), 20, |chapter| {
        let mut analysis = fixture(chapter)?;
        analysis.tokens.clear();
        Ok::<_, Infallible>(analysis)
    });
    assert!(matches!(
        error,
        Err(PasteImportError::Storage(StorageError::InvalidAnalysis))
    ));
    assert_eq!(state(&storage), before);
    assert!(matches!(
        storage.import_analyzed_paste(&text("犬"), -1, must_not_analyze),
        Err(PasteImportError::Storage(StorageError::InvalidVocabulary))
    ));
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER fail_index BEFORE INSERT ON paste_content_hashes
        BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        )
        .unwrap();
    assert!(matches!(
        storage.import_analyzed_paste(&text("犬"), 20, must_not_analyze),
        Err(PasteImportError::Storage(StorageError::Sqlite(_)))
    ));
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_index")
        .unwrap();
    assert_eq!(
        existing(
            storage
                .import_analyzed_paste(&text("猫"), 30, must_not_analyze)
                .unwrap()
        ),
        saved.document
    );
    created(
        storage
            .import_analyzed_paste(&text("犬"), 40, fixture)
            .unwrap(),
    );
}

#[test]
fn deleting_document_removes_identity_and_reimport_starts_fresh_preserving_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = text("猫 猫");
    let imported = created(storage.import_analyzed_paste(&input, 10, fixture).unwrap());
    let doc = &imported.document;
    let cat = Surface::new("猫").unwrap();
    let entry = storage.vocabulary_entry(&cat).unwrap().unwrap();
    let known = storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 20)
        .unwrap();
    let page_length = storage.page_length().unwrap();
    let completion = PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: doc.document_id,
        page: LogicalPages::new(std::slice::from_ref(&doc.chapter), page_length)
            .unwrap()
            .pages()[0],
        page_length,
        departing_position: ReadingPosition {
            source_id: doc.chapter.source_id,
            chapter_id: doc.chapter.chapter_id,
            offset: TextOffset::START,
        },
        completed_at_unix_ms: 30,
        pending_time: Default::default(),
    };
    storage.record_page_completion(completion).unwrap();
    // Test-only deletion; no production delete API or new semantics introduced.
    storage
        .connection
        .execute_batch("BEGIN; DELETE FROM chapter_texts; DELETE FROM documents; COMMIT;")
        .unwrap();
    assert_eq!(count(&storage, "paste_content_hashes"), 0);
    assert_eq!(count(&storage, "reading_completions"), 1);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let fresh = created(storage.import_analyzed_paste(&input, 40, fixture).unwrap());
    assert!(fresh.document.document_id > doc.document_id);
    assert!(fresh.document.chapter.source_id > doc.chapter.source_id);
    assert!(fresh.document.chapter.chapter_id > doc.chapter.chapter_id);
    assert!(fresh.document.reading_position.is_none());
    assert_eq!(fresh.registered_surfaces, 0);
    assert_eq!(storage.vocabulary_entry(&cat).unwrap(), known);
    assert_eq!(
        storage.record_page_completion(completion).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
}

#[test]
fn corrupt_hash_match_errors_instead_of_returning_unrelated_document() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    created(
        storage
            .import_analyzed_paste(&text("猫"), 10, fixture)
            .unwrap(),
    );
    storage
        .connection
        .execute("UPDATE chapter_texts SET text = '犬'", [])
        .unwrap();
    let before = state(&storage);
    assert!(matches!(
        storage.import_analyzed_paste(&text("猫"), 20, must_not_analyze),
        Err(PasteImportError::Storage(StorageError::InvalidDocument))
    ));
    assert_eq!(state(&storage), before);
}

#[test]
fn failed_hash_migration_preserves_version_seven_database_and_can_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..7]).unwrap();
    let mut storage = Storage { connection };
    let saved = storage.create_paste(&text("猫")).unwrap();
    storage
        .connection
        .execute_batch("CREATE TABLE paste_content_hashes_by_digest (fixture TEXT)")
        .unwrap();
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(history(&database.path()).len(), 7);
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection
        .table_exists(None, "paste_content_hashes")
        .unwrap());
    connection
        .execute_batch("DROP TABLE paste_content_hashes_by_digest")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 10);
    assert_eq!(storage.load_paste(saved.document_id).unwrap(), Some(saved));
    assert_eq!(count(&storage, "paste_content_hashes"), 0);
}
