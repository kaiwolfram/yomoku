use std::{convert::Infallible, sync::atomic::AtomicUsize};

use yomoku_core::{
    analysis::ChapterAnalysis,
    completion::AnalyzedCompletionOutcome,
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
fn prepared_import_rechecks_digest_under_the_write_lock_and_skips_losing_analysis() {
    let database = TestDatabase::new();
    let mut loser = Storage::open(database.path()).unwrap();
    let mut winner = Storage::open(database.path()).unwrap();
    let bytes = "猫 犬\n".as_bytes();
    let encoding = Some(yomoku_import::txt::TextEncoding::Utf8);
    let FilePreflight::Prepared(prepared) = loser
        .prepare_txt::<Infallible>(bytes, "loser.txt", encoding)
        .unwrap()
    else {
        panic!("digest must be absent during preflight");
    };
    let TxtImportOutcome::Created(committed) = winner
        .import_analyzed_txt(bytes, "winner.txt", encoding, 10, fixture)
        .unwrap()
    else {
        panic!("second connection must win");
    };
    let before = fs::read(database.path()).unwrap();
    let Some(TxtImportOutcome::Existing(existing)) = loser
        .import_prepared_txt(
            prepared,
            20,
            &ImportCancellation::default(),
            must_not_analyze,
        )
        .unwrap()
    else {
        panic!("losing import must reference the committed document");
    };
    assert_eq!(existing.document_id, committed.document.document_id);
    assert_eq!(existing.title, "winner.txt");
    assert_eq!(count(&loser, "imported_files"), 1);
    assert_eq!(fs::read(database.path()).unwrap(), before);
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
        .save_document_reading_position(imported.document.document_id, position)
        .map(|value| value.map(crate::tests::as_paste))
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
fn analyzer_validation_and_indexing_errors_leave_previous_imports_intact() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let saved = created(
        storage
            .import_analyzed_paste(&text("猫"), 10, fixture)
            .unwrap(),
    );
    storage.unanalyzed_fixture(&text("既存")).unwrap();
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
    let completion = storage
        .prepare_fixture_completion(yomoku_core::completion::PageCompletionIntent {
            analysis_id: imported.analysis_id,
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
        })
        .unwrap();
    storage
        .record_effective_page_completion(&completion)
        .unwrap();
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
        storage
            .record_effective_page_completion(&completion)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
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
