use yomoku_core::{
    completion::{CompletionOutcome, PageCompletion},
    document::{CompletionCommandId, DocumentId, PasteText, PastedDocument, TextOffset},
    pagination::{LogicalPages, PageLength},
};

use super::{positions::position, *};

fn fixture(storage: &mut Storage) -> PastedDocument {
    storage
        .create_paste(
            &PasteText::new(format!(
                "{}\n{}\n終𠮷\0",
                "語".repeat(500),
                "文".repeat(500)
            ))
            .unwrap(),
        )
        .unwrap()
}

fn command(storage: &mut Storage, document: &PastedDocument) -> PageCompletion {
    let page_length = storage.page_length().unwrap();
    let pages = LogicalPages::new(std::slice::from_ref(&document.chapter), page_length).unwrap();
    let departing_position = document.reading_position.unwrap_or(position(document, 0));
    PageCompletion {
        command_id: storage.new_completion_command_id().unwrap(),
        document_id: document.document_id,
        page: pages.pages()[pages.page_at(departing_position).unwrap()],
        page_length,
        departing_position,
        completed_at_unix_ms: 1_792_756_800_123,
        pending_time: Default::default(),
    }
}

fn count(storage: &Storage) -> i64 {
    storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn duplicates_count_once_rereads_count_again_and_previous_only_changes_position() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let first = command(&mut storage, &document);
    assert_eq!(count(&storage), 0); // Issuing an intent is not reading history.
    assert_eq!(
        storage.record_page_completion(first).unwrap(),
        CompletionOutcome::Recorded
    );
    let forward = storage.load_paste(document.document_id).unwrap().unwrap();
    assert_eq!(
        forward.reading_position.unwrap().offset,
        first.page.range.end
    );
    for _ in 0..2 {
        assert_eq!(
            storage.record_page_completion(first).unwrap(),
            CompletionOutcome::AlreadyRecorded
        );
        assert_eq!(
            storage.load_paste(document.document_id).unwrap(),
            Some(forward.clone())
        );
    }
    let previous = storage
        .save_reading_position(document.document_id, position(&document, 0))
        .unwrap()
        .unwrap();
    assert_eq!(count(&storage), 1);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_page_completion(first).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(
        storage.load_paste(document.document_id).unwrap(),
        Some(previous.clone())
    );
    let mut reread = command(&mut storage, &previous);
    assert_ne!(first.command_id, reread.command_id);
    reread.completed_at_unix_ms += 1;
    assert_eq!(
        storage.record_page_completion(reread).unwrap(),
        CompletionOutcome::Recorded
    );
    assert_eq!(count(&storage), 2);
    let dates: Vec<i64> = storage
        .connection
        .prepare("SELECT completed_at_unix_ms FROM reading_completions ORDER BY command_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        dates,
        [first.completed_at_unix_ms, reread.completed_at_unix_ms]
    );
}

#[test]
fn event_position_and_commit_failures_roll_back_together_and_same_id_can_retry() {
    for (stage, sql) in [
        ("event", "CREATE TRIGGER fail_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("position insert", "CREATE TRIGGER fail_completion BEFORE INSERT ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("position update", "CREATE TRIGGER fail_completion BEFORE UPDATE ON reading_positions BEGIN SELECT RAISE(ABORT, 'fixture'); END;"),
        ("commit", "CREATE TABLE deferred_completion_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_completion AFTER INSERT ON reading_completions BEGIN INSERT INTO deferred_completion_failure VALUES (-1); END;"),
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let mut document = fixture(&mut storage);
        if stage == "position update" {
            document = storage.save_reading_position(document.document_id, position(&document, 3)).unwrap().unwrap();
        }
        let completion = command(&mut storage, &document);
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.record_page_completion(completion), Err(StorageError::Sqlite(_))), "{stage}");
        assert_eq!(count(&storage), 0, "{stage}");
        assert_eq!(storage.load_paste(document.document_id).unwrap(), Some(document.clone()), "{stage}");
        storage.connection.execute_batch("DROP TRIGGER fail_completion").unwrap();
        assert_eq!(storage.record_page_completion(completion).unwrap(), CompletionOutcome::Recorded);
        assert_eq!(count(&storage), 1);
    }
}

#[test]
fn rejects_unissued_foreign_partial_split_scalar_stale_and_invalid_date_commands() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let other = fixture(&mut storage);
    let valid = command(&mut storage, &document);
    let mut invalid = vec![
        PageCompletion {
            command_id: CompletionCommandId::try_from(i64::MAX).unwrap(),
            ..valid
        },
        PageCompletion {
            document_id: other.document_id,
            ..valid
        },
        PageCompletion {
            document_id: DocumentId::try_from(i64::MAX).unwrap(),
            ..valid
        },
        PageCompletion {
            completed_at_unix_ms: -1,
            ..valid
        },
        PageCompletion {
            departing_position: position(&document, 3),
            ..valid
        },
        PageCompletion {
            departing_position: position(&other, 0),
            ..valid
        },
        PageCompletion {
            page_length: PageLength::Characters250,
            ..valid
        },
    ];
    for end in [0, 1, 3, i64::MAX] {
        let mut value = valid;
        value.page.range.end = TextOffset::try_from(end).unwrap();
        invalid.push(value);
    }
    let mut value = valid;
    value.page.range.start = valid.page.range.end;
    value.page.range.end = TextOffset::START;
    invalid.push(value);
    value = valid;
    value.page.source_id = other.chapter.source_id;
    invalid.push(value);
    value = valid;
    value.page.chapter_id = other.chapter.chapter_id;
    invalid.push(value);
    for invalid in invalid {
        assert!(
            matches!(
                storage.record_page_completion(invalid),
                Err(StorageError::InvalidCompletion)
            ),
            "{invalid:?}"
        );
        assert_eq!(count(&storage), 0);
        assert_eq!(
            storage.load_paste(document.document_id).unwrap(),
            Some(document.clone())
        );
    }
    // A new preset invalidates an uncommitted page intent; it does not silently
    // complete a differently sized page after reflow.
    storage.save_page_length(PageLength::Characters250).unwrap();
    assert!(matches!(
        storage.record_page_completion(valid),
        Err(StorageError::InvalidCompletion)
    ));
    storage.save_page_length(valid.page_length).unwrap();
    assert_eq!(
        storage.record_page_completion(valid).unwrap(),
        CompletionOutcome::Recorded
    );
    let mut stale = valid;
    stale.command_id = storage.new_completion_command_id().unwrap();
    assert!(matches!(
        storage.record_page_completion(stale),
        Err(StorageError::InvalidCompletion)
    ));
    assert_eq!(count(&storage), 1);
}

#[test]
fn reused_id_requires_identical_frozen_payload_without_moving_progress() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let other = fixture(&mut storage);
    let valid = command(&mut storage, &document);
    storage.record_page_completion(valid).unwrap();
    let saved = storage.load_paste(document.document_id).unwrap();
    let mut conflicts = vec![
        PageCompletion {
            document_id: other.document_id,
            ..valid
        },
        PageCompletion {
            completed_at_unix_ms: valid.completed_at_unix_ms + 1,
            ..valid
        },
        PageCompletion {
            departing_position: position(&document, 3),
            ..valid
        },
        PageCompletion {
            departing_position: position(&other, 0),
            ..valid
        },
        PageCompletion {
            page_length: PageLength::Characters250,
            ..valid
        },
    ];
    let mut conflict = valid;
    conflict.page.range.end = TextOffset::START;
    conflicts.push(conflict);
    conflict = valid;
    conflict.page.range.start = TextOffset::at(&document.chapter.text, 3).unwrap();
    conflicts.push(conflict);
    conflict = valid;
    conflict.page.source_id = other.chapter.source_id;
    conflicts.push(conflict);
    conflict = valid;
    conflict.page.chapter_id = other.chapter.chapter_id;
    conflicts.push(conflict);
    for conflict in conflicts {
        assert!(matches!(
            storage.record_page_completion(conflict),
            Err(StorageError::InvalidCompletion)
        ));
        assert_eq!(storage.load_paste(document.document_id).unwrap(), saved);
        assert_eq!(count(&storage), 1);
    }
    storage.save_page_length(PageLength::Characters250).unwrap();
    assert_eq!(
        storage.record_page_completion(valid).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
}

#[test]
fn concurrent_duplicate_submissions_record_once_and_issue_distinct_ids() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let completion = command(&mut storage, &document);
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let path = database.path();
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    let result = storage.record_page_completion(completion).unwrap();
                    (result, storage.new_completion_command_id().unwrap())
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results
            .iter()
            .filter(|(outcome, _)| *outcome == CompletionOutcome::Recorded)
            .count(),
        1
    );
    let ids: std::collections::HashSet<_> = results.iter().map(|(_, id)| *id).collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(count(&storage), 1);
}

#[test]
fn history_and_replay_survive_live_document_and_text_deletion() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let document = fixture(&mut storage);
    let completion = command(&mut storage, &document);
    storage.record_page_completion(completion).unwrap();
    // Test-only deletion: no production destructive API is introduced.
    storage
        .connection
        .execute_batch("DELETE FROM documents; DELETE FROM chapter_texts;")
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_page_completion(completion).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(count(&storage), 1);
    assert!(storage.list_documents().unwrap().is_empty());
    let original: String = storage
        .connection
        .query_row("SELECT original_title FROM sources", [], |row| row.get(0))
        .unwrap();
    assert_eq!(original, document.title);
    let other = fixture(&mut storage);
    let mut invalid = completion;
    invalid.command_id = storage.new_completion_command_id().unwrap();
    assert!(matches!(
        storage.record_page_completion(invalid),
        Err(StorageError::InvalidCompletion)
    ));
    assert_ne!(other.document_id, document.document_id);
    assert_eq!(count(&storage), 1);
}

#[test]
fn final_page_records_exact_unicode_eof_and_remains_independent_of_finish_state() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut document = fixture(&mut storage);
    for expected_count in 1..=3 {
        let completion = command(&mut storage, &document);
        storage.record_page_completion(completion).unwrap();
        document = storage.load_paste(document.document_id).unwrap().unwrap();
        assert_eq!(count(&storage), expected_count);
    }
    assert_eq!(
        document
            .reading_position
            .unwrap()
            .offset
            .byte_index(&document.chapter.text)
            .unwrap(),
        document.chapter.text.len()
    );
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 100);
    // No checkmark/Finish Book behavior is claimed by this repository foundation.
}

#[test]
fn upgrades_version_four_without_changing_existing_text_settings_or_position() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..4]).unwrap();
    let mut old = Storage { connection };
    let document = fixture(&mut old);
    let expected = old
        .save_reading_position(document.document_id, position(&document, 3))
        .unwrap();
    old.save_page_length(PageLength::Characters750).unwrap();
    drop(old);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 9);
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters750);
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    assert_eq!(count(&storage), 0);
}

#[test]
fn failed_completion_migration_preserves_version_four_and_can_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..4]).unwrap();
    let mut old = Storage { connection };
    let document = fixture(&mut old);
    let expected = old
        .save_reading_position(document.document_id, position(&document, 3))
        .unwrap();
    // Fail after the new command table has been created, within migration 0005.
    old.connection
        .execute_batch("CREATE TABLE reading_completions (fixture TEXT);")
        .unwrap();
    drop(old);
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection
        .table_exists(None, "completion_commands")
        .unwrap());
    assert_eq!(history(&database.path()).len(), 4);
    connection
        .execute_batch("DROP TABLE reading_completions")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    assert_eq!(storage.schema_version().unwrap(), 9);
    assert_eq!(count(&storage), 0);
}

#[test]
fn issued_ids_survive_reopen_and_exhaustion_preserves_history_and_position() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = storage.new_completion_command_id().unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(storage.new_completion_command_id().unwrap() > first);
    let document = fixture(&mut storage);
    let completion = command(&mut storage, &document);
    storage.record_page_completion(completion).unwrap();
    let expected = storage.load_paste(document.document_id).unwrap();
    storage
        .connection
        .execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'completion_commands'",
            [i64::MAX - 1],
        )
        .unwrap();
    assert_eq!(
        i64::from(storage.new_completion_command_id().unwrap()),
        i64::MAX
    );
    assert!(matches!(
        storage.new_completion_command_id(),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(count(&storage), 1);
    assert_eq!(storage.load_paste(document.document_id).unwrap(), expected);
    assert_eq!(
        storage.record_page_completion(completion).unwrap(),
        CompletionOutcome::AlreadyRecorded
    );
}
