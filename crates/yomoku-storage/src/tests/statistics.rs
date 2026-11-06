use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::PasteText,
    effective_reader::EffectivePageCompletion,
    reader::ReaderPage,
    statistics::LifetimeReadingTotals,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

mod buckets;
mod status;

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .analyzed_fixture(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

fn intent(storage: &mut Storage, imported: &AnalyzedPaste) -> PageCompletionIntent {
    let snapshot = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    PageCompletionIntent {
        document_id: snapshot.document.document_id,
        page: page.anchor,
        page_length: snapshot.page_length,
        departing_position: page.position,
        completed_at_unix_ms: 500,
        analysis_id: imported.analysis_id,
    }
}

fn prepare(storage: &mut Storage, imported: &AnalyzedPaste) -> EffectivePageCompletion {
    let intent = intent(storage, imported);
    let key = storage
        .load_effective_reader_tokens(intent.document_id, intent.page)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    storage
        .prepare_effective_page_completion(intent, &key)
        .unwrap()
}

fn assert_totals(storage: &Storage, japanese_occurrences: u64) {
    assert_eq!(
        storage.lifetime_reading_totals().unwrap(),
        LifetimeReadingTotals {
            japanese_occurrences,
        }
    );
}

#[test]
fn original_and_effective_completions_keep_repetitions_rereads_statuses_and_exact_unicode() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(
        &mut storage,
        "猫 猫 食べる 食べた 𠮷 が か\u{3099} English 。 📖",
    );
    for (surface, status) in [
        ("食べる", VocabularyStatus::Learning),
        ("食べた", VocabularyStatus::Familiar),
        ("𠮷", VocabularyStatus::Known),
        ("が", VocabularyStatus::Ignored),
    ] {
        let entry = storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(entry.id, status, 200)
            .unwrap();
    }
    let first_intent = intent(&mut storage, &imported);
    let original = storage.prepare_fixture_completion(first_intent).unwrap();
    storage.record_effective_page_completion(&original).unwrap();
    assert_totals(&storage, 7);
    storage
        .save_document_reading_position(
            imported.document.document_id,
            first_intent.departing_position,
        )
        .unwrap();
    // Replaying old success cannot count the incomplete reread.
    assert_eq!(
        storage.record_effective_page_completion(&original).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 7);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_totals(&storage, 7);
    let reread = prepare(&mut storage, &imported);
    storage.record_effective_page_completion(&reread).unwrap();
    assert_totals(&storage, 14);
    assert_eq!(
        storage.record_effective_page_completion(&reread).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 14);
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_totals(&storage, 14);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn document_deletion_keeps_totals_and_vocabulary_deletion_recalculates_without_json_recount() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let completed = prepare(&mut storage, &imported);
    storage
        .record_effective_page_completion(&completed)
        .unwrap();
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    assert_totals(&storage, 3);
    // Exercise the existing history cascade directly; production confirmed
    // vocabulary deletion remains its own later implementation-plan slice.
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    assert_totals(&storage, 1);
    assert_eq!(
        storage
            .record_effective_page_completion(&completed)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 1);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_totals(&storage, 1);
    let reimported = import(&mut storage, "猫 猫 犬");
    let new_cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    assert_ne!(new_cat.id, cat.id);
    assert_eq!(new_cat.status, VocabularyStatus::New);
    assert_totals(&storage, 1);
    let reimported = prepare(&mut storage, &reimported);
    storage
        .record_effective_page_completion(&reimported)
        .unwrap();
    assert_totals(&storage, 4);
    storage
        .connection
        .execute("DELETE FROM vocabulary", [])
        .unwrap();
    assert_totals(&storage, 0);
}

#[test]
fn failed_completion_is_invisible_and_concurrent_duplicate_success_counts_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let completed = prepare(&mut storage, &imported);
    storage.connection.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences WHEN NEW.token_index = 1 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage
        .record_effective_page_completion(&completed)
        .is_err());
    assert_totals(&storage, 0);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail")
        .unwrap();
    let barrier = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let completed = &completed;
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            scope.spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage.record_effective_page_completion(completed).unwrap();
                assert_totals(&storage, 3);
            });
        }
    });
    assert_totals(&storage, 3);
}

#[test]
fn unknown_event_versions_fail_without_repair() {
    for sql in [
        "UPDATE reading_completions SET semantics_version = 2",
        "UPDATE completion_analyses SET semantics_version = 2",
        "UPDATE effective_completion_commands SET semantics_version = 3",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫");
        let completion = prepare(&mut storage, &imported);
        storage
            .record_effective_page_completion(&completion)
            .unwrap();
        storage
            .connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        storage.connection.execute(sql, []).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(
            matches!(
                storage.lifetime_reading_totals(),
                Err(StorageError::InvalidCompletion)
            ),
            "{sql}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn compact_mode_c_reservation_excludes_other_pages_and_keeps_atomic_deletion_safe_retries() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(
        &mut storage,
        &format!("猫\n{}\nUNRELATED_CHAPTER_MARKER", "English ".repeat(1000)),
    );
    let frozen = prepare(&mut storage, &imported);
    let (version, payload): (i64, String) = storage
        .connection
        .query_row(
            "SELECT semantics_version, payload FROM effective_completion_commands",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(version, 2);
    assert!(!payload.contains("UNRELATED_CHAPTER_MARKER"));
    let record: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(record["mode"], "C");
    assert_eq!(record["analysis_id"], i64::from(imported.analysis_id));
    assert!(record.get("key").is_none());
    assert!(record.get("segmentation").is_none());
    assert_eq!(record["occurrences"].as_array().unwrap().len(), 1);
    assert!(payload.len() < imported.document.chapter.text.len());
    storage.connection.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON reading_occurrences BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage.record_effective_page_completion(&frozen).is_err());
    assert_totals(&storage, 0);

    storage
        .connection
        .execute_batch("DROP TRIGGER fail_receipt")
        .unwrap();
    assert!(matches!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::Recorded { .. }
    ));
    assert_totals(&storage, 1);
    let mode: String = storage
        .connection
        .query_row("SELECT split_mode FROM completion_analyses", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(mode, "c");
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 1);
}
