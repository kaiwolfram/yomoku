use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::{OverrideScope, SegmentationOverride},
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::EffectivePageCompletion,
    reader::ReaderPage,
    statistics::LifetimeReadingTotals,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
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

fn save_time(storage: &mut Storage, imported: &AnalyzedPaste, milliseconds: i64) {
    let position = intent(storage, imported).departing_position;
    let saved = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(
            saved,
            PendingReadingTime::from_millis(milliseconds).unwrap(),
        )
        .unwrap();
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

fn assert_totals(storage: &Storage, japanese_occurrences: u64, active_time_ms: u64) {
    assert_eq!(
        storage.lifetime_reading_totals().unwrap(),
        LifetimeReadingTotals {
            japanese_occurrences,
            active_time_ms
        }
    );
}

#[test]
fn imports_checkpoints_unused_commands_and_legacy_events_are_excluded_and_reads_never_write() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_totals(&storage, 0, 0);
    let imported = import(&mut storage, "猫 猫 𠮷 English 。 📖");
    save_time(&mut storage, &imported, 1234);
    storage.new_completion_command_id().unwrap();
    let pending = prepare(&mut storage, &imported);
    let legacy = import(&mut storage, "犬");
    save_time(&mut storage, &legacy, 9999);
    let legacy_intent = intent(&mut storage, &legacy);
    let legacy = storage.prepare_page_completion(legacy_intent).unwrap();
    storage.record_page_completion(legacy.completion).unwrap();
    // A read-only connection and exact DB bytes protect against cache/registration
    // writes, including when the result is empty despite substantial pending work.
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for _ in 0..3 {
        assert_totals(&storage, 0, 0);
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    storage.record_effective_page_completion(&pending).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert_totals(&storage, 3, 1234);
    assert_eq!(fs::read(database.path()).unwrap(), before);
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
    save_time(&mut storage, &imported, 1234);
    let first_intent = intent(&mut storage, &imported);
    let original = storage.prepare_page_completion(first_intent).unwrap();
    storage
        .record_analyzed_page_completion(original.completion, original.analysis_id)
        .unwrap();
    assert_totals(&storage, 7, 1234);
    storage
        .save_document_reading_position(
            imported.document.document_id,
            first_intent.departing_position,
        )
        .unwrap();
    save_time(&mut storage, &imported, 2000);
    // Replaying old success cannot consume the new incomplete reread's time.
    assert_eq!(
        storage
            .record_analyzed_page_completion(original.completion, original.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 7, 1234);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_totals(&storage, 7, 1234);
    let reread = prepare(&mut storage, &imported);
    storage.record_effective_page_completion(&reread).unwrap();
    assert_totals(&storage, 14, 3234);
    assert_eq!(
        storage.record_effective_page_completion(&reread).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 14, 3234);
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_totals(&storage, 14, 3234);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn unresolved_effective_segments_count_and_zero_japanese_pages_keep_their_time_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた 猫");
    let chapter = &imported.document.chapter;
    storage
        .save_segmentation_override(&SegmentationOverride {
            scope: OverrideScope::AllOccurrences,
            anchor: TextAnchor {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                range: TextRange {
                    start: TextOffset::START,
                    end: TextOffset::at(&chapter.text, "食べた".len()).unwrap(),
                },
            },
            surface: Surface::new("食べた").unwrap(),
            parts: vec![Surface::new("食べ").unwrap(), Surface::new("た").unwrap()],
        })
        .unwrap();
    save_time(&mut storage, &imported, 111);
    let completion = prepare(&mut storage, &imported);
    assert_eq!(completion.occurrences().len(), 5);
    assert!(completion.occurrences()[..4]
        .iter()
        .all(|token| token.metadata.is_none()));
    storage
        .record_effective_page_completion(&completion)
        .unwrap();
    assert_totals(&storage, 5, 111);
    let non_japanese = import(&mut storage, "English 。 📖");
    save_time(&mut storage, &non_japanese, 222);
    let empty = prepare(&mut storage, &non_japanese);
    assert!(empty.occurrences().is_empty());
    storage.record_effective_page_completion(&empty).unwrap();
    assert_totals(&storage, 5, 333);
    // A zero-time Japanese page still contributes every occurrence.
    let zero_time = import(&mut storage, "犬 犬");
    let zero_time = prepare(&mut storage, &zero_time);
    storage
        .record_effective_page_completion(&zero_time)
        .unwrap();
    assert_totals(&storage, 7, 333);
}

#[test]
fn document_deletion_keeps_totals_and_vocabulary_deletion_recalculates_without_json_recount() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    save_time(&mut storage, &imported, 1234);
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
    assert_totals(&storage, 3, 1234);
    // Exercise the existing history cascade directly; production confirmed
    // vocabulary deletion remains its own later implementation-plan slice.
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    assert_totals(&storage, 1, 1234);
    assert_eq!(
        storage
            .record_effective_page_completion(&completed)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_totals(&storage, 1, 1234);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_totals(&storage, 1, 1234);
    let reimported = import(&mut storage, "猫 猫 犬");
    let new_cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    assert_ne!(new_cat.id, cat.id);
    assert_eq!(new_cat.status, VocabularyStatus::New);
    assert_totals(&storage, 1, 1234);
    save_time(&mut storage, &reimported, 100);
    let reimported = prepare(&mut storage, &reimported);
    storage
        .record_effective_page_completion(&reimported)
        .unwrap();
    assert_totals(&storage, 4, 1334);
    storage
        .connection
        .execute("DELETE FROM vocabulary", [])
        .unwrap();
    assert_totals(&storage, 0, 1334);
}

#[test]
fn failed_completion_is_invisible_and_concurrent_duplicate_success_counts_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    save_time(&mut storage, &imported, 1234);
    let completed = prepare(&mut storage, &imported);
    storage.connection.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences WHEN NEW.token_index = 1 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage
        .record_effective_page_completion(&completed)
        .is_err());
    assert_totals(&storage, 0, 0);
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
                assert_totals(&storage, 3, 1234);
            });
        }
    });
    assert_totals(&storage, 3, 1234);
}

#[test]
fn unknown_event_versions_and_corrupt_durations_fail_without_repair() {
    for sql in [
        "UPDATE reading_completions SET semantics_version = 2",
        "UPDATE completion_analyses SET semantics_version = 2",
        "UPDATE effective_completion_commands SET semantics_version = 2",
        "UPDATE reading_completions SET active_time_ms = -1",
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
fn lifetime_time_uses_exact_integers_and_overflow_errors_without_truncating_or_writing() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = import(&mut storage, "猫");
    save_time(&mut storage, &first, i64::MAX);
    let completed = prepare(&mut storage, &first);
    storage
        .record_effective_page_completion(&completed)
        .unwrap();
    assert_totals(&storage, 1, i64::MAX as u64);
    let second = import(&mut storage, "犬");
    save_time(&mut storage, &second, 1);
    let completed = prepare(&mut storage, &second);
    storage
        .record_effective_page_completion(&completed)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.lifetime_reading_totals(),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
}
