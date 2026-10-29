use yomoku_core::{library::WordStatusCounts, statistics::StatusTimelinePoint};

use super::*;

fn point(
    timestamp: i64,
    [new, learning, familiar, known, ignored]: [u64; 5],
) -> StatusTimelinePoint {
    StatusTimelinePoint {
        occurred_at_unix_ms: timestamp,
        counts: WordStatusCounts {
            new,
            learning,
            familiar,
            known,
            ignored,
        },
    }
}

fn status(storage: &mut Storage, surface: &str, status: VocabularyStatus, timestamp: i64) {
    let entry = storage
        .vocabulary_entry(&Surface::new(surface).unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, status, timestamp)
        .unwrap();
}

#[test]
fn import_registration_jumps_preserve_unique_unicode_surfaces_and_all_five_statuses() {
    use VocabularyStatus::*;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
    import(
        &mut storage,
        "猫 猫 食べる 食べた 𠮷 が か\u{3099} English 。 📖",
    );
    for (surface, choice) in [
        ("食べる", Learning),
        ("食べた", Familiar),
        ("𠮷", Known),
        ("が", Ignored),
    ] {
        status(&mut storage, surface, choice, 200);
    }
    let expected = [point(100, [6, 0, 0, 0, 0]), point(200, [2, 1, 1, 1, 1])];
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    // Reimporting already registered surfaces and choosing the same status are
    // not new events, even with a later date. Inflections retain separate status.
    import(&mut storage, "食べた 猫");
    status(&mut storage, "食べた", Familiar, 300);
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    storage
        .create_analyzed_paste(&PasteText::new("犬 猫".into()).unwrap(), 400, fixture)
        .unwrap();
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [expected[0], expected[1], point(400, [3, 1, 1, 1, 1]),]
    );
}

#[test]
fn original_effective_completions_rereads_and_manual_demotions_share_the_same_timeline() {
    use VocabularyStatus::*;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬 𠮷");
    status(&mut storage, "犬", Ignored, 200);
    status(&mut storage, "𠮷", Learning, 200);
    let intent = intent(&mut storage, &imported);
    let completion = storage.prepare_page_completion(intent).unwrap();
    storage
        .record_analyzed_page_completion(completion.completion, completion.analysis_id)
        .unwrap();
    let expected = [
        point(100, [3, 0, 0, 0, 0]),
        point(200, [1, 1, 0, 0, 1]),
        point(500, [0, 1, 0, 1, 1]),
    ];
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    storage
        .record_analyzed_page_completion(completion.completion, completion.analysis_id)
        .unwrap();
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    storage
        .save_document_reading_position(imported.document.document_id, intent.departing_position)
        .unwrap();
    status(&mut storage, "猫", New, 600);
    let mut reread_intent = intent;
    reread_intent.completed_at_unix_ms = 700;
    let key = storage
        .load_effective_reader_tokens(intent.document_id, intent.page)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    let reread = storage
        .prepare_effective_page_completion(reread_intent, &key)
        .unwrap();
    storage.record_effective_page_completion(&reread).unwrap();
    storage.record_effective_page_completion(&reread).unwrap();
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [
            expected[0],
            expected[1],
            expected[2],
            point(600, [1, 1, 0, 0, 1]),
            point(700, [0, 1, 0, 1, 1]),
        ]
    );
}

#[test]
fn document_deletion_preserves_timeline_and_vocabulary_erasure_recalculates_all_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let completed = prepare(&mut storage, &imported);
    storage
        .record_effective_page_completion(&completed)
        .unwrap();
    let expected = [point(100, [2, 0, 0, 0, 0]), point(500, [0, 0, 0, 2, 0])];
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    // Existing FK contract; confirmed vocabulary deletion UI is a later slice.
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    let expected = [point(100, [1, 0, 0, 0, 0]), point(500, [0, 0, 0, 1, 0])];
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    storage
        .record_effective_page_completion(&completed)
        .unwrap();
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    // Only an explicit later import registers the erased surface, with a new ID.
    storage
        .create_analyzed_paste(&PasteText::new("猫 犬".into()).unwrap(), 800, fixture)
        .unwrap();
    let new_cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    assert_ne!(cat.id, new_cat.id);
    assert_eq!(new_cat.status, VocabularyStatus::New);
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [expected[0], expected[1], point(800, [1, 0, 0, 1, 0])]
    );
    storage
        .connection
        .execute("DELETE FROM vocabulary", [])
        .unwrap();
    assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
}

#[test]
fn reads_are_query_only_byte_preserving_and_ignore_pending_and_uncommitted_work() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 犬");
    save_time(&mut storage, &imported, 1234);
    let completion = prepare(&mut storage, &imported);
    let expected = [point(100, [2, 0, 0, 0, 0])];
    let other = Storage::open(database.path()).unwrap();
    other.connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    crate::vocabulary::set_vocabulary_status_in(
        &other.connection,
        storage
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .id,
        VocabularyStatus::Known,
        200,
    )
    .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for _ in 0..3 {
        assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    other.connection.execute_batch("ROLLBACK").unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    storage.connection.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage
        .record_effective_page_completion(&completion)
        .is_err());
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail")
        .unwrap();
    storage
        .record_effective_page_completion(&completion)
        .unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [expected[0], point(500, [0, 0, 0, 2, 0])]
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn frozen_completion_dates_and_regressing_clocks_do_not_break_status_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫");
    let mut frozen_intent = intent(&mut storage, &imported);
    frozen_intent.completed_at_unix_ms = 50;
    let completion = storage.prepare_page_completion(frozen_intent).unwrap();
    storage
        .record_analyzed_page_completion(completion.completion, completion.analysis_id)
        .unwrap();
    status(&mut storage, "猫", VocabularyStatus::Learning, 75);
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [point(100, [0, 1, 0, 0, 0])]
    );
    // Projection leaves raw dates and causes intact, including the frozen command.
    let dates: Vec<i64> = storage
        .connection
        .prepare("SELECT occurred_at_unix_ms FROM vocabulary_status_events ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(dates, [100, 50, 75]);
}

#[test]
fn unread_manual_status_survives_document_deletion_and_erasure_never_rescans_live_content() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた");
    status(&mut storage, "食べた", VocabularyStatus::Known, 200);
    let expected = [point(100, [1, 0, 0, 0, 0]), point(200, [0, 0, 0, 1, 0])];
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    assert_totals(&storage, 0, 0);
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    assert_eq!(storage.vocabulary_status_timeline().unwrap(), expected);
    let live = import(&mut storage, "食べた");
    storage
        .connection
        .execute("DELETE FROM vocabulary", [])
        .unwrap();
    assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
    storage
        .load_document_reader_snapshot(live.document.document_id)
        .unwrap()
        .unwrap();
    let page = intent(&mut storage, &live).page;
    storage
        .load_effective_reader_tokens(live.document.document_id, page)
        .unwrap()
        .unwrap();
    assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
    assert!(storage
        .vocabulary_entry(&Surface::new("食べた").unwrap())
        .unwrap()
        .is_none());
    // Only a newly committed reading recreates the erased entry; registration
    // and New→Known at the same instant project as a single Known point.
    let completion = prepare(&mut storage, &live);
    storage
        .record_effective_page_completion(&completion)
        .unwrap();
    assert_eq!(
        storage.vocabulary_status_timeline().unwrap(),
        [point(500, [0, 0, 0, 1, 0])]
    );
}

#[test]
fn concurrent_duplicate_completion_commits_one_status_transition() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let completion = prepare(&mut storage, &imported);
    let barrier = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            let completion = &completion;
            scope.spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage
                    .record_effective_page_completion(completion)
                    .unwrap();
                assert_eq!(
                    storage.vocabulary_status_timeline().unwrap(),
                    [point(100, [2, 0, 0, 0, 0]), point(500, [0, 0, 0, 2, 0])]
                );
            });
        }
    });
}

#[test]
fn invalid_versions_causes_dates_status_chains_and_completion_provenance_fail_without_repair() {
    for sql in [
        "UPDATE vocabulary_status_events SET semantics_version = 2",
        "UPDATE vocabulary_status_events SET cause = 'future' WHERE cause = 'completion'",
        "UPDATE vocabulary_status_events SET occurred_at_unix_ms = -1",
        "UPDATE vocabulary_status_events SET new_status = 'future' WHERE cause = 'completion'",
        "UPDATE vocabulary_status_events SET old_status = 'learning' WHERE cause = 'completion'",
        "UPDATE vocabulary_status_events SET new_status = 'known' WHERE cause = 'registration'",
        "UPDATE vocabulary_status_events SET completion_command_id = NULL WHERE cause = 'completion'",
        "DELETE FROM vocabulary_status_events WHERE cause = 'registration'",
        "UPDATE reading_completions SET semantics_version = 2",
        "UPDATE completion_analyses SET semantics_version = 2",
        "UPDATE effective_completion_commands SET semantics_version = 2",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫");
        let completion = prepare(&mut storage, &imported);
        storage.record_effective_page_completion(&completion).unwrap();
        storage.connection.pragma_update(None, "ignore_check_constraints", true).unwrap();
        storage.connection.execute(sql, []).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(storage.vocabulary_status_timeline(), Err(StorageError::InvalidVocabulary)), "{sql}");
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}
