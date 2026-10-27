use yomoku_core::{
    analysis::{OverrideScope, SegmentationOverride, SplitMode},
    document::{ChapterText, PasteText, TextAnchor, TextOffset, TextRange},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

fn edit(chapter: &ChapterText, surface: &str, parts: &[&str]) -> SegmentationOverride {
    let start = chapter.text.find(surface).unwrap();
    SegmentationOverride {
        scope: OverrideScope::default(),
        anchor: TextAnchor {
            source_id: chapter.source_id,
            chapter_id: chapter.chapter_id,
            range: TextRange {
                start: TextOffset::at(&chapter.text, start).unwrap(),
                end: TextOffset::at(&chapter.text, start + surface.len()).unwrap(),
            },
        },
        surface: Surface::new(surface).unwrap(),
        parts: parts
            .iter()
            .map(|part| Surface::new(*part).unwrap())
            .collect(),
    }
}

fn count(storage: &Storage, table: &str) -> i64 {
    storage
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn entry(storage: &Storage, surface: &str) -> Option<yomoku_core::vocabulary::VocabularyEntry> {
    storage
        .vocabulary_entry(&Surface::new(surface).unwrap())
        .unwrap()
}

fn state(storage: &Storage) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    // Preserve actual values, not just counters, across failed writes and retry.
    [
        "segmentation_overrides",
        "segmentation_override_parts",
        "vocabulary",
        "vocabulary_status_events",
        "chapter_analyses",
        "analyzed_tokens",
        "reading_occurrences",
        "reading_completions",
        "reading_positions",
        "pending_reading_time",
        "completion_commands",
        "completion_analyses",
        "effective_completion_commands",
    ]
    .into_iter()
    .map(|table| {
        let mut statement = storage
            .connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
            .unwrap()
            .collect::<Result<Vec<Vec<_>>, _>>()
            .unwrap()
    })
    .collect()
}

#[test]
fn registered_override_registers_distinct_parts_preserving_all_existing_statuses() {
    for status in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた 食べた 未読語");
        import(&mut storage, "食べ");
        let existing = entry(&storage, "食べ").unwrap();
        storage
            .set_vocabulary_status(existing.id, status, 110)
            .unwrap();
        let existing = entry(&storage, "食べ").unwrap();
        let original = storage.chapter_analysis(imported.analysis_id).unwrap();
        let reader = storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap();
        let events = count(&storage, "vocabulary_status_events");
        let rule = edit(&imported.document.chapter, "食べた", &["食べ", "た"]);
        let (id, registered) = storage
            .save_registered_segmentation_override(&rule, 120)
            .unwrap();
        assert_eq!(registered, 1);
        assert_eq!(entry(&storage, "食べ").unwrap(), existing);
        let created = entry(&storage, "た").unwrap();
        assert_eq!(created.status, VocabularyStatus::New);
        assert_eq!(created.registered_at_unix_ms, 120);
        assert_eq!(count(&storage, "vocabulary_status_events"), events + 1);
        assert_eq!(
            storage.chapter_analysis(imported.analysis_id).unwrap(),
            original
        );
        assert_eq!(
            storage
                .load_document_reader_snapshot(imported.document.document_id)
                .unwrap(),
            reader
        );
        for table in [
            "reading_occurrences",
            "reading_completions",
            "completion_commands",
            "reading_positions",
            "pending_reading_time",
        ] {
            assert_eq!(count(&storage, table), 0, "{table}");
        }
        assert!(storage.reverse_segmentation_override(id).unwrap());
        assert_eq!(entry(&storage, "た").unwrap(), created);
        assert_eq!(entry(&storage, "食べ").unwrap(), existing);
    }
}

#[test]
fn registered_override_retry_is_write_free_after_reopen_and_vocabulary_deletion() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫猫 猫猫");
    let rule = edit(&imported.document.chapter, "猫猫", &["猫", "猫"]);
    let (id, registered) = storage
        .save_registered_segmentation_override(&rule, 120)
        .unwrap();
    assert_eq!(registered, 1);
    let created = entry(&storage, "猫").unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(created.id)],
        )
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let before = state(&storage);
    let bytes = fs::read(database.path()).unwrap();
    storage.connection.execute_batch("CREATE TEMP TRIGGER no_override_write BEFORE UPDATE ON segmentation_overrides BEGIN SELECT RAISE(ABORT, 'unexpected write'); END;
        CREATE TEMP TRIGGER no_registration BEFORE INSERT ON vocabulary BEGIN SELECT RAISE(ABORT, 'unexpected registration'); END;").unwrap();
    let mut second_occurrence = rule.clone();
    second_occurrence.anchor.range.start =
        TextOffset::at(&imported.document.chapter.text, "猫猫 ".len()).unwrap();
    second_occurrence.anchor.range.end = TextOffset::at(
        &imported.document.chapter.text,
        imported.document.chapter.text.len(),
    )
    .unwrap();
    assert_eq!(
        storage
            .save_registered_segmentation_override(&second_occurrence, 999)
            .unwrap(),
        (id, 0)
    );
    assert_eq!(state(&storage), before);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert!(entry(&storage, "猫").is_none());
    storage
        .connection
        .execute_batch("DROP TRIGGER no_override_write; DROP TRIGGER no_registration;")
        .unwrap();
    storage.reverse_segmentation_override(id).unwrap();
    let (new_id, registered) = storage
        .save_registered_segmentation_override(&rule, 1000)
        .unwrap();
    assert_ne!(new_id, id);
    assert_eq!(registered, 1);
    assert_ne!(entry(&storage, "猫").unwrap().id, created.id);
}

#[test]
fn registered_override_respects_local_precedence_and_exact_non_japanese_metadata() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた");
    let chapter = &imported.document.chapter;
    let mut alternate = fixture(chapter).unwrap();
    alternate.mode = SplitMode::A;
    let mut first = alternate.tokens[0].clone();
    first.surface = "食べ".into();
    first.anchor.range.end = TextOffset::at(&chapter.text, "食べ".len()).unwrap();
    let mut second = alternate.tokens[0].clone();
    second.surface = "た".into();
    second.anchor.range.start = first.anchor.range.end;
    second.is_japanese = false;
    alternate.tokens.splice(0..1, [first, second]);
    storage
        .store_chapter_analysis(chapter.chapter_id, &alternate)
        .unwrap();
    let global = edit(chapter, "食べた", &["食べ", "た"]);
    let mut local = edit(chapter, "食べた", &["食べた"]);
    local.scope = OverrideScope::ThisOccurrence;
    let local_id = storage
        .save_registered_segmentation_override(&local, 120)
        .unwrap()
        .0;
    // A global rule hidden by a local choice at the selected occurrence does
    // not register hidden parts or scan other occurrences/documents.
    let (global_id, registered) = storage
        .save_registered_segmentation_override(&global, 130)
        .unwrap();
    assert_eq!(registered, 0);
    assert!(entry(&storage, "食べ").is_none());
    assert!(entry(&storage, "た").is_none());
    storage.reverse_segmentation_override(local_id).unwrap();
    storage.reverse_segmentation_override(global_id).unwrap();
    let (id, registered) = storage
        .save_registered_segmentation_override(&global, 140)
        .unwrap();
    assert_eq!(registered, 1);
    assert!(entry(&storage, "食べ").is_some());
    assert!(entry(&storage, "た").is_none());
    let resolved = storage
        .load_effective_chapter_segmentation(chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert_eq!(resolved.segments[0].boundary.override_id, Some(id));
    assert!(!resolved.segments[1].metadata.as_ref().unwrap().is_japanese);
    assert!(resolved.segments[3].metadata.is_none());
}

#[test]
fn registered_override_rejects_missing_foreign_non_japanese_gapped_and_corrupt_input() {
    for case in 0..10 {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた 猫 English");
        let mut rule = edit(&imported.document.chapter, "食べた", &["食べ", "た"]);
        let mut timestamp = 120;
        match case {
            0 => timestamp = -1,
            1 => rule.anchor.source_id = 999.try_into().unwrap(),
            2 => rule.anchor.range.start = 1.try_into().unwrap(),
            3 => rule.parts = vec![Surface::new("別").unwrap()],
            4 => rule = edit(&imported.document.chapter, "English", &["Eng", "lish"]),
            5 => rule = edit(&imported.document.chapter, "食べた 猫", &["食べた ", "猫"]),
            6 => {
                storage
                    .connection
                    .execute("DELETE FROM chapter_analyses", [])
                    .unwrap();
            }
            7 => {
                storage
                    .connection
                    .execute(
                        "UPDATE analyzed_tokens SET surface = '別' WHERE token_index = 1",
                        [],
                    )
                    .unwrap();
            }
            8 => {
                let mut a = fixture(&imported.document.chapter).unwrap();
                a.mode = SplitMode::A;
                let id = storage
                    .store_chapter_analysis(imported.document.chapter.chapter_id, &a)
                    .unwrap();
                storage.connection.execute("UPDATE analyzed_tokens SET reading = ' ' WHERE analysis_id = ?1 AND token_index = 2", [i64::from(id)]).unwrap();
            }
            9 => {
                storage.save_segmentation_override(&rule).unwrap();
                storage.connection.execute("UPDATE segmentation_override_parts SET surface = '別' WHERE part_index = 0", []).unwrap();
            }
            _ => unreachable!(),
        }
        let before = state(&storage);
        assert!(
            storage
                .save_registered_segmentation_override(&rule, timestamp)
                .is_err(),
            "case {case}"
        );
        assert_eq!(state(&storage), before, "case {case}");
    }
}

#[test]
fn registered_override_write_and_commit_failures_roll_back_replacement_and_registration() {
    for trigger in [
        "CREATE TEMP TRIGGER fail AFTER UPDATE ON segmentation_overrides BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON segmentation_overrides BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON segmentation_override_parts WHEN NEW.part_index = 1 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON vocabulary WHEN NEW.surface = '食べ' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON vocabulary_status_events WHEN NEW.occurred_at_unix_ms = 120 AND (SELECT surface FROM vocabulary WHERE id = NEW.vocabulary_id) = '食べ' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TEMP TRIGGER fail AFTER INSERT ON vocabulary_status_events WHEN NEW.occurred_at_unix_ms = 120 BEGIN INSERT INTO deferred_child VALUES (99); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "食べた");
        let previous = storage.save_registered_segmentation_override(&edit(&imported.document.chapter, "食べた", &["食べた"]), 110).unwrap().0;
        let rule = edit(&imported.document.chapter, "食べた", &["食べ", "た"]);
        storage.connection.execute_batch("CREATE TEMP TABLE deferred_parent (id INTEGER PRIMARY KEY);
            CREATE TEMP TABLE deferred_child (id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        storage.connection.execute_batch(trigger).unwrap();
        let before = state(&storage);
        assert!(matches!(storage.save_registered_segmentation_override(&rule, 120), Err(StorageError::Sqlite(_))), "{trigger}");
        assert_eq!(state(&storage), before, "{trigger}");
        assert!(storage.segmentation_override(previous).unwrap().unwrap().active);
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        assert_eq!(storage.save_registered_segmentation_override(&rule, 120).unwrap().1, 2);
    }
}

#[test]
fn registered_override_unicode_partitions_are_literal_and_concurrent_retries_register_once() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "𠮷か\u{3099}\0猫 𠮷か\u{3099}\0猫");
    let rule = edit(
        &imported.document.chapter,
        "𠮷か\u{3099}\0猫",
        &["𠮷", "か\u{3099}", "\0猫"],
    );
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let path = database.path();
            let rule = rule.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage
                    .save_registered_segmentation_override(&rule, 120)
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results[0].0, results[1].0);
    assert_eq!(results.iter().map(|r| r.1).sum::<usize>(), 3);
    assert_eq!(count(&storage, "segmentation_overrides"), 1);
    for part in &rule.parts {
        let created = storage.vocabulary_entry(part).unwrap().unwrap();
        assert_eq!(created.surface, *part);
        assert_eq!(created.status, VocabularyStatus::New);
        assert_eq!(created.registered_at_unix_ms, 120);
    }
    let resolved = storage
        .load_effective_chapter_segmentation(imported.document.chapter.chapter_id)
        .unwrap()
        .unwrap();
    assert!(resolved.segments.iter().all(|s| s.metadata.is_none()));
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .load_effective_chapter_segmentation(imported.document.chapter.chapter_id)
            .unwrap()
            .unwrap(),
        resolved
    );
}

#[test]
fn registered_override_preserves_completed_history_progress_and_pending_time() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        completion::{AnalyzedPageCompletion, PageCompletion},
        pagination::{LogicalPages, ReadingPosition},
    };
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "食べた 食べた 未読語");
    let chapter = &imported.document.chapter;
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::START,
    };
    let saved = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(1234).unwrap())
        .unwrap();
    let page_length = storage.page_length().unwrap();
    let page = LogicalPages::new(std::slice::from_ref(chapter), page_length)
        .unwrap()
        .pages()[0];
    let completion = AnalyzedPageCompletion {
        analysis_id: imported.analysis_id,
        completion: PageCompletion {
            command_id: storage.new_completion_command_id().unwrap(),
            document_id: imported.document.document_id,
            page,
            page_length,
            departing_position: position,
            completed_at_unix_ms: 110,
            pending_time: PendingReadingTime::from_millis(1234).unwrap(),
        },
    };
    storage
        .record_analyzed_page_completion(completion.completion, completion.analysis_id)
        .unwrap();
    // A reread restores its own pending time without changing past occurrences.
    storage
        .save_reading_position(imported.document.document_id, position)
        .unwrap();
    let saved = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(567).unwrap())
        .unwrap();
    let deleted = entry(&storage, "未読語").unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM vocabulary WHERE id = ?1",
            [i64::from(deleted.id)],
        )
        .unwrap();
    let before = state(&storage);
    let reader = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap();
    let rule = edit(chapter, "食べた", &["食べ", "た"]);
    let (id, registered) = storage
        .save_registered_segmentation_override(&rule, 120)
        .unwrap();
    assert_eq!(registered, 2);
    assert!(entry(&storage, "未読語").is_none());
    let after = state(&storage);
    // Only the first four tables (override + parts + registration/status events)
    // may change. The nonempty histories/time/position remain byte-for-byte rows.
    assert_eq!(&after[4..], &before[4..]);
    assert_eq!(
        storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap(),
        reader
    );
    assert!(reader.unwrap().document.finished_at_least_once);
    assert_eq!(
        storage
            .load_pending_reading_time(imported.document.document_id, position)
            .unwrap()
            .pending(),
        PendingReadingTime::from_millis(567).unwrap()
    );
    storage.reverse_segmentation_override(id).unwrap();
    assert_eq!(&state(&storage)[4..], &before[4..]);
}
