use yomoku_core::{
    document::{PasteText, VocabularyId},
    vocabulary::{Surface, VocabularyStatus},
};

use super::*;

fn surface(text: &str) -> Surface {
    Surface::new(text).unwrap()
}

fn counts(storage: &Storage) -> (i64, i64) {
    storage.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM vocabulary), (SELECT COUNT(*) FROM vocabulary_status_events)",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap()
}

type Event = (Option<String>, String, String, i64);

fn events(storage: &Storage, id: VocabularyId) -> Vec<Event> {
    storage
        .connection
        .prepare(
            "SELECT old_status, new_status, cause, occurred_at_unix_ms
         FROM vocabulary_status_events WHERE vocabulary_id = ?1 ORDER BY id",
        )
        .unwrap()
        .query_map([i64::from(id)], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn registers_unique_exact_surfaces_without_encounters_and_reopens() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = [
        "食べる",
        "食べた",
        "食べます",
        "生",
        "食べる",
        "生",
        "が",
        "か\u{3099}",
        "𠮷",
        "語\0尾",
    ]
    .map(surface);
    assert_eq!(storage.register_vocabulary(&input, 0).unwrap(), 8);
    assert_eq!(counts(&storage), (8, 8));
    let expected: Vec<_> = input
        .iter()
        .map(|s| storage.vocabulary_entry(s).unwrap().unwrap())
        .collect();
    for entry in &expected {
        assert_eq!(entry.status, VocabularyStatus::New);
        assert_eq!(entry.registered_at_unix_ms, 0);
        assert_eq!(
            events(&storage, entry.id),
            [(None, "new".into(), "registration".into(), 0)]
        );
    }
    let completed: i64 = storage
        .connection
        .query_row("SELECT COUNT(*) FROM reading_completions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(completed, 0);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.register_vocabulary(&input, 100).unwrap(), 0);
    for (written, expected) in input.iter().zip(expected) {
        assert_eq!(storage.vocabulary_entry(written).unwrap(), Some(expected));
    }
    assert_eq!(counts(&storage), (8, 8));
}

#[test]
fn all_statuses_survive_registration_from_another_document_and_deletion() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = ["新しい", "学ぶ", "慣れる", "知る", "無視"].map(surface);
    let text = PasteText::new("新しい 学ぶ 慣れる 知る 無視".into()).unwrap();
    storage.create_paste(&text).unwrap();
    storage.register_vocabulary(&input, 10).unwrap();
    let statuses = [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ];
    let expected: Vec<_> = input
        .iter()
        .zip(statuses)
        .map(|(s, status)| {
            let entry = storage.vocabulary_entry(s).unwrap().unwrap();
            storage
                .set_vocabulary_status(entry.id, status, 20)
                .unwrap()
                .unwrap()
        })
        .collect();
    let second = storage.create_paste(&text).unwrap();
    assert_eq!(storage.register_vocabulary(&input, 30).unwrap(), 0);
    assert_eq!(counts(&storage), (5, 9));
    // Simulate the future document deletion; no production delete operation.
    storage
        .connection
        .execute_batch("DELETE FROM chapter_texts; DELETE FROM documents;")
        .unwrap();
    assert!(storage.load_paste(second.document_id).unwrap().is_none());
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    for (s, entry) in input.iter().zip(expected) {
        assert_eq!(storage.vocabulary_entry(s).unwrap(), Some(entry));
    }
    assert_eq!(counts(&storage), (5, 9));
}

#[test]
fn lookup_and_unchanged_selection_add_no_events_and_missing_id_is_not_registered() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let s = surface("食べる");
    assert_eq!(storage.vocabulary_entry(&s).unwrap(), None);
    assert_eq!(
        storage
            .set_vocabulary_status(
                VocabularyId::try_from(i64::MAX).unwrap(),
                VocabularyStatus::Known,
                1
            )
            .unwrap(),
        None
    );
    assert_eq!(counts(&storage), (0, 0));
    storage
        .register_vocabulary(std::slice::from_ref(&s), 10)
        .unwrap();
    let entry = storage.vocabulary_entry(&s).unwrap().unwrap();
    assert_eq!(
        storage
            .set_vocabulary_status(entry.id, VocabularyStatus::New, 20)
            .unwrap(),
        Some(entry.clone())
    );
    assert_eq!(storage.vocabulary_entry(&s).unwrap(), Some(entry));
    assert_eq!(counts(&storage), (1, 1));
}

#[test]
fn status_history_records_each_actual_transition_and_explicit_dates() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let s = surface("食べる");
    storage
        .register_vocabulary(std::slice::from_ref(&s), 10)
        .unwrap();
    let original = storage.vocabulary_entry(&s).unwrap().unwrap();
    let statuses = [
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ];
    for (index, status) in statuses.into_iter().enumerate() {
        let updated = storage
            .set_vocabulary_status(original.id, status, 20 + index as i64)
            .unwrap()
            .unwrap();
        assert_eq!(updated.id, original.id);
        assert_eq!(updated.registered_at_unix_ms, 10);
        assert_eq!(updated.status, status);
    }
    assert_eq!(
        events(&storage, original.id),
        [
            (None, "new".into(), "registration".into(), 10),
            (Some("new".into()), "learning".into(), "manual".into(), 20),
            (
                Some("learning".into()),
                "familiar".into(),
                "manual".into(),
                21
            ),
            (Some("familiar".into()), "known".into(), "manual".into(), 22),
            (Some("known".into()), "ignored".into(), "manual".into(), 23),
            (Some("ignored".into()), "new".into(), "manual".into(), 24),
        ]
    );
    drop(storage);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .vocabulary_entry(&s)
            .unwrap(),
        Some(original)
    );
}

#[test]
fn registration_failures_roll_back_the_entire_batch_and_can_retry() {
    for sql in [
        "CREATE TRIGGER fail_vocabulary BEFORE INSERT ON vocabulary WHEN NEW.surface = '失敗' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail_vocabulary BEFORE INSERT ON vocabulary_status_events WHEN NEW.vocabulary_id = 3 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_vocab_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_vocabulary AFTER INSERT ON vocabulary_status_events BEGIN INSERT INTO deferred_vocab_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let existing = surface("保存");
        storage.register_vocabulary(std::slice::from_ref(&existing), 10).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        let input = [surface("途中"), surface("失敗")];
        assert!(matches!(storage.register_vocabulary(&input, 20), Err(StorageError::Sqlite(_))));
        assert_eq!(counts(&storage), (1, 1));
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(counts(&storage), (1, 1));
        storage.connection.execute_batch("DROP TRIGGER fail_vocabulary").unwrap();
        assert_eq!(storage.register_vocabulary(&input, 20).unwrap(), 2);
        assert_eq!(counts(&storage), (3, 3));
    }
}

#[test]
fn status_and_event_and_commit_failures_preserve_current_status_and_history() {
    for sql in [
        "CREATE TRIGGER fail_vocabulary BEFORE UPDATE ON vocabulary BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail_vocabulary BEFORE INSERT ON vocabulary_status_events BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_vocab_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_vocabulary AFTER INSERT ON vocabulary_status_events BEGIN INSERT INTO deferred_vocab_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let s = surface("保存");
        storage.register_vocabulary(std::slice::from_ref(&s), 10).unwrap();
        let original = storage.vocabulary_entry(&s).unwrap().unwrap();
        storage.connection.execute_batch(sql).unwrap();
        assert!(matches!(storage.set_vocabulary_status(original.id, VocabularyStatus::Known, 20), Err(StorageError::Sqlite(_))));
        assert_eq!(storage.vocabulary_entry(&s).unwrap(), Some(original.clone()));
        assert_eq!(counts(&storage), (1, 1));
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.vocabulary_entry(&s).unwrap(), Some(original.clone()));
        storage.connection.execute_batch("DROP TRIGGER fail_vocabulary").unwrap();
        storage.set_vocabulary_status(original.id, VocabularyStatus::Known, 20).unwrap();
        assert_eq!(counts(&storage), (1, 2));
    }
}

#[test]
fn concurrent_registration_and_same_status_selection_record_once() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    let barrier = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = database.path();
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    let s = surface("同じ");
                    let created = storage
                        .register_vocabulary(std::slice::from_ref(&s), 10)
                        .unwrap();
                    let entry = storage.vocabulary_entry(&s).unwrap().unwrap();
                    storage
                        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 20)
                        .unwrap();
                    created
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .sum::<usize>(),
            1
        );
    });
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(counts(&storage), (1, 2));
    assert_eq!(
        storage
            .vocabulary_entry(&surface("同じ"))
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Known
    );
}

#[test]
fn rejects_negative_dates_and_corrupt_entries_without_repair() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let s = surface("保存");
    assert!(matches!(
        storage.register_vocabulary(std::slice::from_ref(&s), -1),
        Err(StorageError::InvalidVocabulary)
    ));
    assert_eq!(counts(&storage), (0, 0));
    storage
        .register_vocabulary(std::slice::from_ref(&s), 10)
        .unwrap();
    let entry = storage.vocabulary_entry(&s).unwrap().unwrap();
    assert!(matches!(
        storage.set_vocabulary_status(entry.id, VocabularyStatus::Known, -1),
        Err(StorageError::InvalidVocabulary)
    ));
    assert_eq!(storage.vocabulary_entry(&s).unwrap(), Some(entry.clone()));
    storage
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    for sql in [
        "UPDATE vocabulary SET status = 'invalid', registered_at_unix_ms = 10",
        "UPDATE vocabulary SET status = 'new', registered_at_unix_ms = -1",
    ] {
        storage.connection.execute(sql, []).unwrap();
        assert!(matches!(
            storage.vocabulary_entry(&s),
            Err(StorageError::InvalidVocabulary)
        ));
        assert!(matches!(
            storage.register_vocabulary(std::slice::from_ref(&s), 30),
            Err(StorageError::InvalidVocabulary)
        ));
        assert!(matches!(
            storage.set_vocabulary_status(entry.id, VocabularyStatus::Known, 30),
            Err(StorageError::InvalidVocabulary)
        ));
        assert_eq!(counts(&storage), (1, 1));
    }
}

#[test]
fn vocabulary_deletion_cascades_only_its_history_and_reregistration_gets_fresh_id() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = [surface("削除"), surface("保存")];
    storage.register_vocabulary(&input, 10).unwrap();
    let deleted = storage.vocabulary_entry(&input[0]).unwrap().unwrap();
    let kept = storage.vocabulary_entry(&input[1]).unwrap().unwrap();
    storage
        .set_vocabulary_status(deleted.id, VocabularyStatus::Known, 20)
        .unwrap();
    // Schema verification only: production vocabulary deletion requires confirmation.
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(deleted.id)],
        )
        .unwrap();
    assert_eq!(counts(&storage), (1, 1));
    assert!(events(&storage, deleted.id).is_empty());
    assert_eq!(storage.vocabulary_entry(&input[1]).unwrap(), Some(kept));
    assert_eq!(storage.vocabulary_entry(&input[0]).unwrap(), None);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.vocabulary_entry(&input[0]).unwrap(), None);
    storage.register_vocabulary(&input, 30).unwrap();
    let fresh = storage.vocabulary_entry(&input[0]).unwrap().unwrap();
    assert!(fresh.id > deleted.id);
    assert_eq!(fresh.status, VocabularyStatus::New);
    assert_eq!(fresh.registered_at_unix_ms, 30);
    assert_eq!(
        storage
            .set_vocabulary_status(deleted.id, VocabularyStatus::Known, 40)
            .unwrap(),
        None
    );
    assert_eq!(counts(&storage), (2, 2));
}

#[test]
fn upgrades_version_five_preserving_documents_settings_position_and_completions() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..5]).unwrap();
    let mut old = Storage { connection };
    let doc = old
        .create_paste(&PasteText::new("日本語".into()).unwrap())
        .unwrap();
    let command = yomoku_core::completion::PageCompletion {
        command_id: old.new_completion_command_id().unwrap(),
        document_id: doc.document_id,
        page: yomoku_core::pagination::LogicalPages::new(
            std::slice::from_ref(&doc.chapter),
            old.page_length().unwrap(),
        )
        .unwrap()
        .pages()[0],
        page_length: old.page_length().unwrap(),
        departing_position: yomoku_core::pagination::ReadingPosition {
            source_id: doc.chapter.source_id,
            chapter_id: doc.chapter.chapter_id,
            offset: yomoku_core::document::TextOffset::START,
        },
        completed_at_unix_ms: 10,
    };
    old.record_page_completion(command).unwrap();
    let expected = old.load_paste(doc.document_id).unwrap();
    old.save_page_length(yomoku_core::pagination::PageLength::Characters750)
        .unwrap();
    drop(old);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 6);
    assert_eq!(storage.load_paste(doc.document_id).unwrap(), expected);
    assert_eq!(
        storage.page_length().unwrap(),
        yomoku_core::pagination::PageLength::Characters750
    );
    assert_eq!(
        storage.record_page_completion(command).unwrap(),
        yomoku_core::completion::CompletionOutcome::AlreadyRecorded
    );
    assert_eq!(counts(&storage), (0, 0));
}

#[test]
fn failed_vocabulary_migration_preserves_version_five_and_can_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..5]).unwrap();
    connection
        .execute_batch("CREATE TABLE vocabulary_status_events (fixture TEXT)")
        .unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let connection = Connection::open(database.path()).unwrap();
    assert!(!connection.table_exists(None, "vocabulary").unwrap());
    assert_eq!(history(&database.path()).len(), 5);
    connection
        .execute_batch("DROP TABLE vocabulary_status_events")
        .unwrap();
    drop(connection);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .schema_version()
            .unwrap(),
        6
    );
}
