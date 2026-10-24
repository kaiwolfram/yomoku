use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::SplitMode,
    completion::{AnalyzedCompletionOutcome, PageCompletion},
    document::{AnalysisId, PasteText, PastedDocument},
    pagination::LogicalPages,
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture as analyze, positions::position, *};

type OccurrenceColumns = (i64, String, i64, i64, Option<String>, Option<String>);

fn create_import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, analyze)
        .unwrap()
}

fn command(storage: &mut Storage, document: &PastedDocument) -> PageCompletion {
    let document = storage.load_paste(document.document_id).unwrap().unwrap();
    let page_length = storage.page_length().unwrap();
    let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length).unwrap();
    let departing_position = document.reading_position.unwrap_or(position(&document, 0));
    let page = pages.pages()[pages.page_at(departing_position).unwrap()];
    PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page,
        page_length,
        departing_position,
        completed_at_unix_ms: 500,
        pending_time: storage
            .pending_page_time(document.document_id, page, page_length)
            .unwrap(),
    }
}

fn count(storage: &Storage, table: &str) -> i64 {
    storage
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

fn totals(storage: &Storage) -> Vec<i64> {
    [
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "vocabulary",
        "vocabulary_status_events",
        "reading_positions",
    ]
    .map(|table| count(storage, table))
    .to_vec()
}

fn entry(storage: &Storage, surface: &str) -> yomoku_core::vocabulary::VocabularyEntry {
    storage
        .vocabulary_entry(&Surface::new(surface).unwrap())
        .unwrap()
        .unwrap()
}

fn save_time(storage: &mut Storage, document: &PastedDocument, ms: i64) -> SavedReadingTime {
    let token = storage
        .load_pending_reading_time(document.document_id, position(document, 0))
        .unwrap();
    storage
        .save_pending_reading_time(token, PendingReadingTime::from_millis(ms).unwrap())
        .unwrap()
}

#[test]
fn repetitions_exact_surfaces_all_statuses_and_metadata_commit_with_time_and_position() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .create_analyzed_paste(
            &PasteText::new("猫 猫 食べた 食べる 𠮷 が か\u{3099} English 。 📖 \0".into())
                .unwrap(),
            100,
            |chapter| {
                let mut analysis = analyze(chapter).unwrap();
                analysis.tokens[2].reading = Some("タベタ".into());
                analysis.tokens[2].dictionary_form = Some(Surface::new("食べる").unwrap());
                Ok::<_, std::convert::Infallible>(analysis)
            },
        )
        .unwrap();
    for (surface, status) in [
        ("食べた", VocabularyStatus::Learning),
        ("食べる", VocabularyStatus::Familiar),
        ("𠮷", VocabularyStatus::Known),
        ("が", VocabularyStatus::Ignored),
    ] {
        storage
            .set_vocabulary_status(entry(&storage, surface).id, status, 200)
            .unwrap();
    }
    let checkpoint = save_time(&mut storage, &imported.document, 1234);
    let completion = command(&mut storage, &imported.document);
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
    );
    assert_eq!(count(&storage, "reading_occurrences"), 7);
    let rows: Vec<OccurrenceColumns> = storage.connection.prepare(
        "SELECT token_index, surface, start_byte, end_byte, reading, dictionary_form FROM reading_occurrences ORDER BY token_index"
    ).unwrap().query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))
        .unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(
        rows.iter().map(|r| r.1.as_str()).collect::<Vec<_>>(),
        ["猫", "猫", "食べた", "食べる", "𠮷", "が", "か\u{3099}"]
    );
    for row in &rows {
        assert_eq!(
            &imported.document.chapter.text[row.2 as usize..row.3 as usize],
            row.1
        );
    }
    assert_eq!(rows[2].4.as_deref(), Some("タベタ"));
    assert_eq!(rows[2].5.as_deref(), Some("食べる"));
    let events: Vec<(String, String, String, i64, i64)> = storage.connection.prepare(
        "SELECT v.surface, e.old_status, e.new_status, e.occurred_at_unix_ms, e.completion_command_id
         FROM vocabulary_status_events e JOIN vocabulary v ON v.id = e.vocabulary_id WHERE cause = 'completion' ORDER BY e.id"
    ).unwrap().query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
        .unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(events.len(), 2);
    for (surface, old, new, date, id) in events {
        assert!(["猫", "か\u{3099}"].contains(&surface.as_str()));
        assert_eq!(
            (old.as_str(), new.as_str(), date, id),
            ("new", "known", 500, i64::from(completion.command_id))
        );
    }
    for (surface, status) in [
        ("猫", VocabularyStatus::Known),
        ("食べた", VocabularyStatus::Learning),
        ("食べる", VocabularyStatus::Familiar),
        ("𠮷", VocabularyStatus::Known),
        ("が", VocabularyStatus::Ignored),
    ] {
        assert_eq!(entry(&storage, surface).status, status);
    }
    assert_eq!(
        storage
            .connection
            .query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        1234
    );
    assert_eq!(
        storage
            .load_pending_reading_time(imported.document.document_id, checkpoint.position())
            .unwrap()
            .pending()
            .milliseconds(),
        0
    );
    assert_eq!(
        storage
            .load_paste(imported.document.document_id)
            .unwrap()
            .unwrap()
            .reading_position
            .unwrap()
            .offset,
        completion.page.range.end
    );
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(count(&storage, "reading_occurrences"), 7);
    assert_eq!(entry(&storage, "猫").status, VocabularyStatus::Known);
}

#[test]
fn replay_after_reopen_reflow_and_reread_never_moves_progress_or_drains_new_time() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "猫 猫");
    save_time(&mut storage, &imported.document, 1000);
    let first = command(&mut storage, &imported.document);
    storage
        .record_analyzed_page_completion(first, imported.analysis_id)
        .unwrap();
    storage
        .save_reading_position(
            imported.document.document_id,
            position(&imported.document, 0),
        )
        .unwrap();
    save_time(&mut storage, &imported.document, 2000);
    let saved = storage.load_paste(imported.document.document_id).unwrap();
    let before = totals(&storage);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .save_page_length(yomoku_core::pagination::PageLength::Characters250)
        .unwrap();
    assert_eq!(
        storage
            .record_analyzed_page_completion(first, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(totals(&storage), before);
    assert_eq!(
        storage.load_paste(imported.document.document_id).unwrap(),
        saved
    );
    assert_eq!(
        storage
            .pending_page_time(
                imported.document.document_id,
                first.page,
                yomoku_core::pagination::PageLength::Characters250
            )
            .unwrap()
            .milliseconds(),
        2000
    );
    let reread = command(&mut storage, &imported.document);
    assert_eq!(
        storage
            .record_analyzed_page_completion(reread, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
    );
    assert_eq!(count(&storage, "reading_occurrences"), 4);
    assert_eq!(
        storage
            .connection
            .query_row(
                "SELECT SUM(active_time_ms) FROM reading_completions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        3000
    );
    assert_eq!(
        storage
            .connection
            .query_row(
                "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'completion'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn concurrent_duplicate_commands_create_one_set_of_occurrences_and_promotions() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "猫 猫 𠮷");
    let completion = command(&mut storage, &imported.document);
    let barrier = Arc::new(Barrier::new(4));
    let outcomes = std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .record_analyzed_page_completion(completion, imported.analysis_id)
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == AnalyzedCompletionOutcome::Recorded { newly_known: 2 })
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == AnalyzedCompletionOutcome::AlreadyRecorded)
            .count(),
        3
    );
    assert_eq!(count(&storage, "reading_occurrences"), 3);
    assert_eq!(count(&storage, "vocabulary_status_events"), 4);
}

#[test]
fn every_write_and_deferred_commit_failure_roll_back_all_completion_effects() {
    for (stage, sql) in [
        ("history", "CREATE TRIGGER fail BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("analysis", "CREATE TRIGGER fail BEFORE INSERT ON completion_analyses BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("registration", "CREATE TRIGGER fail BEFORE INSERT ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("registration event", "CREATE TRIGGER fail BEFORE INSERT ON vocabulary_status_events WHEN NEW.cause = 'registration' BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("status", "CREATE TRIGGER fail BEFORE UPDATE ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("status event", "CREATE TRIGGER fail BEFORE INSERT ON vocabulary_status_events WHEN NEW.cause = 'completion' BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("second occurrence", "CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences WHEN NEW.token_index = 1 BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("time", "CREATE TRIGGER fail BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("position insert", "CREATE TRIGGER fail BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("position update", "CREATE TRIGGER fail BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("finish marker", "CREATE TRIGGER fail BEFORE UPDATE OF finished_at_least_once ON documents BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("commit", "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER INSERT ON reading_occurrences BEGIN INSERT INTO deferred_failure VALUES (-1); END;"),
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = create_import(&mut storage, "猫 猫 𠮷");
        // Exercise recreation of a deleted vocabulary entry only on actual reading.
        storage.connection.execute("DELETE FROM vocabulary WHERE surface = '𠮷'", []).unwrap();
        let token = save_time(&mut storage, &imported.document, 1234);
        if stage == "position update" {
            storage.save_reading_position(imported.document.document_id, position(&imported.document, 0)).unwrap();
        }
        let completion = command(&mut storage, &imported.document);
        let before = totals(&storage);
        let saved = storage.load_paste(imported.document.document_id).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.record_analyzed_page_completion(completion, imported.analysis_id), Err(StorageError::Sqlite(_))), "{stage}");
        assert_eq!(totals(&storage), before, "{stage}");
        assert_eq!(storage.finished_at_least_once(imported.document.document_id).unwrap(), Some(false), "{stage}");
        assert_eq!(entry(&storage, "猫").status, VocabularyStatus::New, "{stage}");
        assert_eq!(storage.load_paste(imported.document.document_id).unwrap(), saved, "{stage}");
        assert_eq!(storage.load_pending_reading_time(imported.document.document_id, token.position()).unwrap(), token, "{stage}");
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        assert_eq!(storage.record_analyzed_page_completion(completion, imported.analysis_id).unwrap(), AnalyzedCompletionOutcome::Recorded { newly_known: 2 }, "{stage}");
        assert_eq!(count(&storage, "reading_occurrences"), 3);
        assert_eq!(storage.finished_at_least_once(imported.document.document_id).unwrap(), Some(true), "{stage}");
    }
}

#[test]
fn frozen_analysis_identity_rejects_missing_foreign_corrupt_and_changed_snapshots() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "猫 猫");
    let other = create_import(&mut storage, "猫 猫");
    let completion = command(&mut storage, &imported.document);
    let before = totals(&storage);
    for invalid in [AnalysisId::try_from(i64::MAX).unwrap(), other.analysis_id] {
        assert!(matches!(
            storage.record_analyzed_page_completion(completion, invalid),
            Err(StorageError::InvalidAnalysis)
        ));
        assert_eq!(totals(&storage), before);
    }
    storage.connection.execute("UPDATE analyzed_tokens SET surface = '違う' WHERE analysis_id = ?1 AND token_index = 0", [i64::from(imported.analysis_id)]).unwrap();
    assert!(matches!(
        storage.record_analyzed_page_completion(completion, imported.analysis_id),
        Err(StorageError::InvalidAnalysis)
    ));
    assert_eq!(totals(&storage), before);
    storage
        .connection
        .execute(
            "UPDATE analyzed_tokens SET surface = '猫' WHERE analysis_id = ?1 AND token_index = 0",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    let mut alternate = analyze(&imported.document.chapter).unwrap();
    alternate.mode = SplitMode::A;
    let alternate = storage
        .store_chapter_analysis(imported.document.chapter.chapter_id, &alternate)
        .unwrap();
    storage
        .record_analyzed_page_completion(completion, imported.analysis_id)
        .unwrap();
    assert!(matches!(
        storage.record_analyzed_page_completion(completion, alternate),
        Err(StorageError::InvalidCompletion)
    ));
    assert!(matches!(
        storage.record_page_completion(completion),
        Err(StorageError::InvalidCompletion)
    ));
    // Legacy events cannot be upgraded by replaying an old ID with new semantics.
    let legacy = command(&mut storage, &other.document);
    storage.record_page_completion(legacy).unwrap();
    assert!(matches!(
        storage.record_analyzed_page_completion(legacy, other.analysis_id),
        Err(StorageError::InvalidCompletion)
    ));
}

#[test]
fn document_deletion_retains_occurrences_but_vocabulary_deletion_erases_only_its_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "猫 猫 𠮷");
    let completion = command(&mut storage, &imported.document);
    storage
        .record_analyzed_page_completion(completion, imported.analysis_id)
        .unwrap();
    storage
        .connection
        .execute_batch("DELETE FROM documents; DELETE FROM chapter_texts;")
        .unwrap();
    assert_eq!(count(&storage, "chapter_analyses"), 0);
    assert_eq!(count(&storage, "reading_occurrences"), 3);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    assert_eq!(count(&storage, "reading_occurrences"), 1);
    assert_eq!(count(&storage, "vocabulary_status_events"), 2);
    assert_eq!(count(&storage, "reading_completions"), 1);
    assert_eq!(count(&storage, "completion_analyses"), 1);
    // Replays do not recreate deleted vocabulary or encounters.
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(count(&storage, "vocabulary"), 1);
}

#[test]
fn completion_reregisters_only_deleted_surfaces_on_the_departing_page() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, &format!("{}\n犬", "猫 ".repeat(500)));
    let old_id = entry(&storage, "猫").id;
    storage
        .connection
        .execute_batch("DELETE FROM vocabulary")
        .unwrap();
    let completion = command(&mut storage, &imported.document);
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
    );
    assert_eq!(count(&storage, "reading_occurrences"), 500);
    let recreated = entry(&storage, "猫");
    assert_ne!(recreated.id, old_id);
    assert_eq!(
        recreated.registered_at_unix_ms,
        completion.completed_at_unix_ms
    );
    assert_eq!(recreated.status, VocabularyStatus::Known);
    assert!(storage
        .vocabulary_entry(&Surface::new("犬").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn non_japanese_completion_has_explicit_analyzed_identity_without_status_changes() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "English 。 📖 \0");
    let completion = command(&mut storage, &imported.document);
    assert_eq!(
        storage
            .record_analyzed_page_completion(completion, imported.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
    );
    assert_eq!(count(&storage, "completion_analyses"), 1);
    assert_eq!(count(&storage, "reading_occurrences"), 0);
    assert_eq!(count(&storage, "vocabulary_status_events"), 0);
}

#[test]
fn migration_preserves_v9_history_and_status_event_id_high_water_mark() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..9]).unwrap();
    let mut storage = Storage { connection };
    let imported = create_import(&mut storage, "猫");
    let id = entry(&storage, "猫").id;
    storage
        .set_vocabulary_status(id, VocabularyStatus::Learning, 200)
        .unwrap();
    // Deliberately retain a sequence higher than every remaining event ID.
    storage
        .connection
        .execute(
            "UPDATE sqlite_sequence SET seq = 100 WHERE name = 'vocabulary_status_events'",
            [],
        )
        .unwrap();
    let old_events = count(&storage, "vocabulary_status_events");
    // The old foundation cannot run its new replay query before migration; insert
    // the exact historical event shape instead.
    let completion = command(&mut storage, &imported.document);
    storage
        .connection
        .execute(
            "INSERT INTO reading_completions VALUES (?1, 1, ?2, ?3, ?4, 0, ?5, 0, 500, 300, 0)",
            (
                i64::from(completion.command_id),
                i64::from(completion.document_id),
                i64::from(completion.page.source_id),
                i64::from(completion.page.chapter_id),
                i64::from(completion.page.range.end),
            ),
        )
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 11);
    assert_eq!(count(&storage, "vocabulary_status_events"), old_events);
    assert_eq!(count(&storage, "completion_analyses"), 0);
    assert_eq!(count(&storage, "reading_completions"), 1);
    storage
        .set_vocabulary_status(id, VocabularyStatus::New, 400)
        .unwrap();
    assert_eq!(
        storage
            .connection
            .query_row("SELECT MAX(id) FROM vocabulary_status_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        101
    );
    let completion = command(&mut storage, &imported.document);
    storage
        .record_analyzed_page_completion(completion, imported.analysis_id)
        .unwrap();
    assert_eq!(entry(&storage, "猫").status, VocabularyStatus::Known);
    assert_eq!(
        storage
            .connection
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert!(storage
        .connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn migration_retains_status_event_sequence_when_all_old_events_were_deleted() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..9]).unwrap();
    let mut storage = Storage { connection };
    create_import(&mut storage, "猫");
    storage
        .connection
        .execute_batch("DELETE FROM vocabulary")
        .unwrap();
    assert_eq!(count(&storage, "vocabulary_status_events"), 0);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .register_vocabulary(&[Surface::new("犬").unwrap()], 200)
        .unwrap();
    assert_eq!(
        storage
            .connection
            .query_row("SELECT id FROM vocabulary_status_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn failed_migration_rolls_back_table_rebuild_indexes_and_history_then_retries() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..9]).unwrap();
    let mut storage = Storage { connection };
    create_import(&mut storage, "猫");
    // Fail after the status table has been rebuilt, late in migration 0010.
    storage
        .connection
        .execute_batch("CREATE TABLE vocabulary_completion_status_once (fixture TEXT)")
        .unwrap();
    drop(storage);
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    let connection = Connection::open(database.path()).unwrap();
    assert_eq!(history(&database.path()).len(), 9);
    assert!(!connection
        .table_exists(None, "completion_analyses")
        .unwrap());
    assert!(!connection
        .table_exists(None, "vocabulary_status_events_next")
        .unwrap());
    assert_eq!(
        connection
            .query_row("SELECT cause FROM vocabulary_status_events", [], |r| r
                .get::<_, String>(
                0
            ))
            .unwrap(),
        "registration"
    );
    connection
        .execute_batch("DROP TABLE vocabulary_completion_status_once")
        .unwrap();
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 11);
    assert_eq!(entry(&storage, "猫").status, VocabularyStatus::New);
}

#[test]
fn preparation_freezes_all_page_buckets_and_reserves_only_after_successful_validation() {
    use yomoku_core::completion::PageCompletionIntent;
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = create_import(&mut storage, "猫 猫 犬");
    let document = &imported.document;
    save_time(&mut storage, document, 100);
    let second = position(document, "猫 ".len());
    storage
        .save_reading_position(document.document_id, second)
        .unwrap();
    let saved = storage
        .load_pending_reading_time(document.document_id, second)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(200).unwrap())
        .unwrap();
    let pages = LogicalPages::new(
        std::slice::from_ref(&document.chapter),
        storage.page_length().unwrap(),
    )
    .unwrap();
    let intent = PageCompletionIntent {
        document_id: document.document_id,
        page: pages.pages()[0],
        page_length: storage.page_length().unwrap(),
        departing_position: second,
        completed_at_unix_ms: 1234,
        analysis_id: imported.analysis_id,
    };
    let before = totals(&storage);
    for invalid in [
        PageCompletionIntent {
            completed_at_unix_ms: -1,
            ..intent
        },
        PageCompletionIntent {
            departing_position: position(document, 0),
            ..intent
        },
        PageCompletionIntent {
            page_length: yomoku_core::pagination::PageLength::Characters250,
            ..intent
        },
    ] {
        assert!(storage.prepare_page_completion(invalid).is_err());
        assert_eq!(count(&storage, "completion_commands"), 0);
        assert_eq!(totals(&storage), before);
    }
    storage.connection.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON completion_commands BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage.prepare_page_completion(intent).is_err());
    assert_eq!(count(&storage, "completion_commands"), 0);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail")
        .unwrap();
    let prepared = storage.prepare_page_completion(intent).unwrap();
    assert_eq!(prepared.completion.pending_time.milliseconds(), 300);
    assert_eq!(prepared.analysis_id, imported.analysis_id);
    assert_eq!(prepared.completion.completed_at_unix_ms, 1234);
    assert_eq!(totals(&storage), before); // No completion, position, statuses or consumption.
    assert_eq!(count(&storage, "completion_commands"), 1);
    assert_eq!(
        storage
            .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
}
