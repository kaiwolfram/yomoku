use yomoku_core::{
    active_time::{ActiveReadingTime, PendingReadingTime, IDLE_TIMEOUT_MS},
    completion::{CompletionOutcome, PageCompletion},
    document::{PasteText, PastedDocument, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
};

use super::{positions::position, *};

fn duration(milliseconds: i64) -> PendingReadingTime {
    PendingReadingTime::from_millis(milliseconds).unwrap()
}

fn fixture(storage: &mut Storage) -> PastedDocument {
    storage
        .create_paste(
            &PasteText::new(format!(
                "{}\n{}\n終𠮷📖\0",
                "語".repeat(500),
                "文".repeat(500)
            ))
            .unwrap(),
        )
        .unwrap()
}

fn pages(document: &PastedDocument, length: PageLength) -> Vec<yomoku_core::document::TextAnchor> {
    LogicalPages::new(std::slice::from_ref(&document.chapter), length)
        .unwrap()
        .pages()
        .to_vec()
}

fn checkpoint(
    storage: &mut Storage,
    document: &PastedDocument,
    offset: usize,
    milliseconds: i64,
) -> SavedReadingTime {
    let at = position(document, offset);
    storage
        .save_reading_position(document.document_id, at)
        .unwrap()
        .unwrap();
    let previous = storage
        .load_pending_reading_time(document.document_id, at)
        .unwrap();
    storage
        .save_pending_reading_time(previous, duration(milliseconds))
        .unwrap()
}

fn completion(storage: &mut Storage, document: &PastedDocument) -> PageCompletion {
    let document = storage.load_paste(document.document_id).unwrap().unwrap();
    let page_length = storage.page_length().unwrap();
    let departing_position = document.reading_position.unwrap_or(position(&document, 0));
    let layout = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length).unwrap();
    let page = layout.pages()[layout.page_at(departing_position).unwrap()];
    PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page,
        page_length,
        departing_position,
        completed_at_unix_ms: 1_792_756_800_123,
        pending_time: storage
            .pending_page_time(document.document_id, page, page_length)
            .unwrap(),
    }
}

fn committed_time(storage: &Storage) -> i64 {
    storage
        .connection
        .query_row(
            "SELECT COALESCE(SUM(active_time_ms), 0) FROM reading_completions",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn checkpoints_reopen_with_fresh_clock_epoch_and_never_create_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let initial = storage
        .load_pending_reading_time(document.document_id, position(&document, 0))
        .unwrap();
    let mut timer = ActiveReadingTime::from_pending(initial.pending());
    timer.resume(90_000).unwrap();
    let saved = storage
        .save_pending_reading_time(initial, timer.checkpoint(91_234).unwrap())
        .unwrap();
    assert_eq!(
        storage
            .save_pending_reading_time(initial, duration(1_234))
            .unwrap(),
        saved
    );
    let saved = storage
        .save_pending_reading_time(saved, timer.pause(92_000).unwrap())
        .unwrap();
    assert_eq!(saved.pending(), duration(2_000));
    assert_eq!(committed_time(&storage), 0);
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM vocabulary_status_events", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(document.clone())
    );
    drop(storage);

    let mut storage = Storage::open(database.path()).unwrap();
    let restored = storage
        .load_pending_reading_time(document.document_id, position(&document, 0))
        .unwrap();
    assert_eq!(restored, saved);
    // Simulated reboot: only the duration survives; no old monotonic epoch or
    // time spent away is restored. Actual Android lifecycle remains separate.
    let mut timer = ActiveReadingTime::from_pending(restored.pending());
    assert_eq!(timer.checkpoint(0).unwrap(), duration(2_000));
    timer.resume(7).unwrap();
    assert_eq!(
        timer.pause(7 + IDLE_TIMEOUT_MS + 99_000).unwrap(),
        duration(302_000)
    );
    storage
        .save_pending_reading_time(restored, timer.checkpoint(500_000).unwrap())
        .unwrap();
    assert_eq!(committed_time(&storage), 0);
}

#[test]
fn stale_decreasing_foreign_and_split_scalar_checkpoints_are_rejected() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let other = fixture(&mut storage);
    let at = position(&document, 0);
    let initial = storage
        .load_pending_reading_time(document.document_id, at)
        .unwrap();
    let saved = storage
        .save_pending_reading_time(initial, duration(100))
        .unwrap();
    let latest = storage
        .save_pending_reading_time(saved, duration(200))
        .unwrap();
    for (previous, value) in [(initial, 100), (initial, 999), (saved, 99), (latest, 199)] {
        assert!(matches!(
            storage.save_pending_reading_time(previous, duration(value)),
            Err(StorageError::InvalidReadingTime)
        ));
    }
    let split = ReadingPosition {
        offset: TextOffset::try_from(1).unwrap(),
        ..at
    };
    for invalid in [
        split,
        position(&other, 0),
        ReadingPosition {
            offset: TextOffset::try_from(i64::MAX).unwrap(),
            ..at
        },
    ] {
        assert!(matches!(
            storage.load_pending_reading_time(document.document_id, invalid),
            Err(StorageError::InvalidReadingTime)
        ));
    }
    storage
        .save_reading_position(document.document_id, position(&document, 3))
        .unwrap();
    assert!(matches!(
        storage.save_pending_reading_time(latest, duration(300)),
        Err(StorageError::InvalidReadingTime)
    ));
    assert_eq!(
        storage
            .load_pending_reading_time(document.document_id, at)
            .unwrap(),
        latest
    );
}

#[test]
fn all_reflow_pairs_preserve_offsets_and_assign_each_duration_once() {
    for before in PageLength::ALL {
        for after in PageLength::ALL {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            let document = fixture(&mut storage);
            storage.save_page_length(before).unwrap();
            let boundary = pages(&document, PageLength::Characters500)[0]
                .range
                .end
                .byte_index(&document.chapter.text)
                .unwrap();
            let offsets = [0, 3, boundary, document.chapter.text.len()];
            for (index, offset) in offsets.into_iter().enumerate() {
                checkpoint(&mut storage, &document, offset, (index as i64 + 1) * 100);
            }
            let original = storage.load_paste(document.document_id).unwrap();
            storage.save_page_length(after).unwrap();
            assert_eq!(storage.load_paste(document.document_id).unwrap(), original);
            let layout = LogicalPages::new(std::slice::from_ref(&document.chapter), after).unwrap();
            let mut total = 0;
            for (index, page) in layout.pages().iter().enumerate() {
                let expected: i64 = offsets
                    .into_iter()
                    .enumerate()
                    .filter(|(_, offset)| {
                        layout.page_at(position(&document, *offset)).unwrap() == index
                    })
                    .map(|(i, _)| (i as i64 + 1) * 100)
                    .sum();
                let actual = storage
                    .pending_page_time(document.document_id, *page, after)
                    .unwrap()
                    .milliseconds();
                assert_eq!(actual, expected, "{before:?} → {after:?} page {index}");
                total += actual;
            }
            assert_eq!(total, 1_000);
            assert_eq!(committed_time(&storage), 0);
        }
    }
}

#[test]
fn completion_consumes_only_its_page_and_duplicate_cannot_drain_reread_time() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let layout = pages(&document, PageLength::default());
    let boundary = layout[0]
        .range
        .end
        .byte_index(&document.chapter.text)
        .unwrap();
    checkpoint(&mut storage, &document, boundary, 2_000);
    let old = checkpoint(&mut storage, &document, 0, 1_000);
    let first = completion(&mut storage, &document);
    assert_eq!(first.pending_time, duration(1_000));
    assert_eq!(
        storage.record_page_completion(first).unwrap(),
        CompletionOutcome::Recorded
    );
    assert_eq!(committed_time(&storage), 1_000);
    assert_eq!(
        storage
            .pending_page_time(document.document_id, layout[1], first.page_length)
            .unwrap(),
        duration(2_000)
    );
    storage
        .save_reading_position(document.document_id, position(&document, 0))
        .unwrap();
    assert!(matches!(
        storage.save_pending_reading_time(old, duration(1_500)),
        Err(StorageError::InvalidReadingTime)
    ));
    let reread = checkpoint(&mut storage, &document, 0, 700);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_page_completion(first).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage
            .load_pending_reading_time(document.document_id, position(&document, 0))
            .unwrap(),
        reread
    );
    assert_eq!(
        storage
            .load_paste(document.document_id)
            .unwrap()
            .unwrap()
            .reading_position,
        Some(position(&document, 0))
    );
    let conflict = PageCompletion {
        pending_time: duration(999),
        ..first
    };
    assert!(matches!(
        storage.record_page_completion(conflict),
        Err(StorageError::InvalidCompletion)
    ));
    let second = completion(&mut storage, &document);
    assert_eq!(second.pending_time, duration(700));
    storage.record_page_completion(second).unwrap();
    assert_eq!(committed_time(&storage), 1_700);
}

#[test]
fn zero_checkpoint_retry_cannot_mistake_completion_for_a_successful_save() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let initial = storage
        .load_pending_reading_time(document.document_id, position(&document, 0))
        .unwrap();
    let command = completion(&mut storage, &document);
    storage.record_page_completion(command).unwrap();
    storage
        .save_reading_position(document.document_id, position(&document, 0))
        .unwrap();
    assert!(matches!(
        storage.save_pending_reading_time(initial, duration(0)),
        Err(StorageError::InvalidReadingTime)
    ));
    assert!(matches!(
        storage.save_pending_reading_time(initial, duration(100)),
        Err(StorageError::InvalidReadingTime)
    ));
}

#[test]
fn changed_pending_total_rejects_a_frozen_intent_without_consuming_time() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let saved = checkpoint(&mut storage, &document, 0, 100);
    let frozen = completion(&mut storage, &document);
    let latest = storage
        .save_pending_reading_time(saved, duration(200))
        .unwrap();
    assert!(matches!(
        storage.record_page_completion(frozen),
        Err(StorageError::InvalidCompletion)
    ));
    assert_eq!(committed_time(&storage), 0);
    assert_eq!(
        storage
            .load_pending_reading_time(document.document_id, position(&document, 0))
            .unwrap(),
        latest
    );
    let fresh = completion(&mut storage, &document);
    storage.record_page_completion(fresh).unwrap();
    assert_eq!(committed_time(&storage), 200);
}

#[test]
fn checkpoint_statement_and_commit_failures_preserve_retry_token() {
    for sql in [
        "CREATE TRIGGER fail_time BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_time_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_time AFTER UPDATE ON pending_reading_time BEGIN INSERT INTO deferred_time_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = fixture(&mut storage);
        let saved = checkpoint(&mut storage, &document, 0, 100);
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.save_pending_reading_time(saved, duration(200)), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.load_pending_reading_time(document.document_id, position(&document, 0)).unwrap(), saved);
        storage.connection.execute_batch("DROP TRIGGER fail_time").unwrap();
        storage.save_pending_reading_time(saved, duration(200)).unwrap();
        assert_eq!(committed_time(&storage), 0);
    }
}

#[test]
fn completion_event_consumption_position_and_commit_failures_roll_back_together() {
    for sql in [
        "CREATE TRIGGER fail_time BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail_time BEFORE UPDATE ON pending_reading_time BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail_time BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_time_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_time AFTER UPDATE ON pending_reading_time BEGIN INSERT INTO deferred_time_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = fixture(&mut storage);
        let saved = checkpoint(&mut storage, &document, 0, 123);
        let expected = storage.load_paste(document.document_id).unwrap();
        let frozen = completion(&mut storage, &document);
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.record_page_completion(frozen), Err(StorageError::Sqlite(_))));
        assert_eq!(committed_time(&storage), 0);
        assert_eq!(storage.load_pending_reading_time(document.document_id, position(&document, 0)).unwrap(), saved);
        assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
        storage.connection.execute_batch("DROP TRIGGER fail_time").unwrap();
        storage.record_page_completion(frozen).unwrap();
        assert_eq!(committed_time(&storage), 123);
    }
}

#[test]
fn concurrent_duplicate_checkpoints_and_completions_count_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let initial = storage
        .load_pending_reading_time(document.document_id, position(&document, 0))
        .unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let saved = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .save_pending_reading_time(initial, duration(123))
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(saved.iter().all(|s| *s == saved[0]));
    let frozen = completion(&mut storage, &document);
    let outcomes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage.record_page_completion(frozen).unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == CompletionOutcome::Recorded)
            .count(),
        1
    );
    assert_eq!(committed_time(&storage), 123);
}

#[test]
fn eof_time_counts_on_final_page_and_deletion_discards_pending_but_retains_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    checkpoint(&mut storage, &document, document.chapter.text.len(), 789);
    let frozen = completion(&mut storage, &document);
    assert_eq!(frozen.pending_time, duration(789));
    storage.record_page_completion(frozen).unwrap();
    checkpoint(&mut storage, &document, document.chapter.text.len(), 111);
    storage
        .connection
        .execute_batch("DELETE FROM documents; DELETE FROM chapter_texts;")
        .unwrap();
    assert_eq!(
        storage
            .connection
            .query_row("SELECT COUNT(*) FROM pending_reading_time", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        storage.record_page_completion(frozen).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(committed_time(&storage), 789);
    let replacement = fixture(&mut storage);
    assert_eq!(
        storage
            .load_pending_reading_time(replacement.document_id, position(&replacement, 0))
            .unwrap()
            .pending(),
        duration(0)
    );
}

#[test]
fn duration_and_revision_overflow_leave_pending_and_history_unchanged() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    checkpoint(&mut storage, &document, 3, 1);
    checkpoint(&mut storage, &document, 0, i64::MAX);
    let page = pages(&document, PageLength::default())[0];
    assert!(matches!(
        storage.pending_page_time(document.document_id, page, PageLength::default()),
        Err(StorageError::InvalidReadingTime)
    ));
    storage
        .connection
        .execute(
            "UPDATE pending_reading_time SET milliseconds = 0, revision = ?1",
            [i64::MAX],
        )
        .unwrap();
    let saved = storage
        .load_pending_reading_time(document.document_id, position(&document, 0))
        .unwrap();
    assert!(matches!(
        storage.save_pending_reading_time(saved, duration(1)),
        Err(StorageError::InvalidReadingTime)
    ));
    let frozen = completion(&mut storage, &document);
    assert!(matches!(
        storage.record_page_completion(frozen),
        Err(StorageError::InvalidReadingTime)
    ));
    assert_eq!(committed_time(&storage), 0);
    assert_eq!(
        storage
            .load_pending_reading_time(document.document_id, position(&document, 0))
            .unwrap(),
        saved
    );
}

#[test]
fn migration_preserves_v8_data_and_rolls_back_schema_and_history_on_failure() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..8]).unwrap();
    let mut old = Storage { connection };
    let document = fixture(&mut old);
    let expected = legacy_position(&old, &document, position(&document, 3));
    old.save_page_length(PageLength::Characters750).unwrap();
    old.connection
        .execute_batch(
            "ALTER TABLE reading_completions ADD COLUMN active_time_ms INTEGER NOT NULL DEFAULT 0;",
        )
        .unwrap();
    drop(old);
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection
        .table_exists(None, "pending_reading_time")
        .unwrap());
    assert_eq!(history(&database.path()).len(), 8);
    connection
        .execute_batch("ALTER TABLE reading_completions DROP COLUMN active_time_ms;")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 11);
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
    assert_eq!(
        storage
            .load_pending_reading_time(document.document_id, position(&document, 3))
            .unwrap()
            .pending(),
        duration(0)
    );
    assert_eq!(committed_time(&storage), 0);
}
