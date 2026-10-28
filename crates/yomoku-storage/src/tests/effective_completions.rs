use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::{OverrideScope, SegmentationOverride, SplitMode},
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    effective_reader::{EffectivePageCompletion, EffectiveReaderTokenKey},
    pagination::{LogicalPages, ReadingPosition},
    vocabulary::{Surface, VocabularyStatus},
};

use super::{imports::fixture, *};

fn setup(storage: &mut Storage, text: &str) -> (AnalyzedPaste, i64) {
    let imported = storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap();
    let chapter = &imported.document.chapter;
    let rule = storage
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
            parts: ["食べ", "た"].map(|s| Surface::new(s).unwrap()).to_vec(),
        })
        .unwrap();
    (imported, i64::from(rule))
}

fn selection(
    storage: &mut Storage,
    imported: &AnalyzedPaste,
) -> (PageCompletionIntent, EffectiveReaderTokenKey) {
    let document = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap()
        .document;
    let page_length = storage.page_length().unwrap();
    let position = document.current_position().unwrap();
    let pages = LogicalPages::new(&document.chapters, page_length).unwrap();
    let page = pages.pages()[pages.page_at(position).unwrap()];
    let key = storage
        .load_effective_reader_tokens(document.document_id, page)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    (
        PageCompletionIntent {
            document_id: document.document_id,
            page,
            page_length,
            departing_position: position,
            completed_at_unix_ms: 500,
            analysis_id: imported.analysis_id,
        },
        key,
    )
}

fn prepare(storage: &mut Storage, imported: &AnalyzedPaste) -> EffectivePageCompletion {
    let (intent, key) = selection(storage, imported);
    storage
        .prepare_effective_page_completion(intent, &key)
        .unwrap()
}

fn rows(storage: &Storage, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = storage
        .connection
        .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| {
            (0..columns)
                .map(|i| row.get(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn state(storage: &Storage) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "reading_positions",
        "documents",
        "pending_reading_time",
        "completion_commands",
        "effective_completion_commands",
    ]
    .map(|table| rows(storage, table))
    .to_vec()
}

fn surfaces(storage: &Storage) -> Vec<(String, Option<String>, Option<String>)> {
    storage.connection.prepare("SELECT surface, reading, dictionary_form FROM reading_occurrences ORDER BY command_id, token_index").unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap().collect::<Result<_, _>>().unwrap()
}

#[test]
fn effective_completions_freeze_repetitions_and_commit_registration_status_time_position_and_finish(
) {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, _) = setup(&mut storage, "食べた 食べた 猫 English 。 📖");
    for (word, status) in [
        ("食べ", VocabularyStatus::Learning),
        ("た", VocabularyStatus::Ignored),
    ] {
        let surface = Surface::new(word).unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&surface), 110)
            .unwrap();
        let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
        storage
            .set_vocabulary_status(entry.id, status, 120)
            .unwrap();
    }
    let position = ReadingPosition {
        source_id: imported.document.chapter.source_id,
        chapter_id: imported.document.chapter.chapter_id,
        offset: TextOffset::START,
    };
    let checkpoint = storage
        .load_pending_reading_time(imported.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(checkpoint, PendingReadingTime::from_millis(1234).unwrap())
        .unwrap();
    let before = state(&storage);
    let frozen = prepare(&mut storage, &imported);
    let after = state(&storage);
    assert_eq!(&before[..10], &after[..10]); // preparation only reserves an ID/snapshot
    assert_eq!(frozen.occurrences().len(), 5);
    assert!(frozen.occurrences()[..4]
        .iter()
        .all(|token| token.metadata.is_none()));
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 1 }
    );
    assert_eq!(
        surfaces(&storage)
            .iter()
            .map(|r| r.0.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た", "食べ", "た", "猫"]
    );
    assert!(surfaces(&storage)
        .iter()
        .all(|r| r.1.is_none() && r.2.is_none()));
    for (word, status) in [
        ("食べ", VocabularyStatus::Learning),
        ("た", VocabularyStatus::Ignored),
        ("猫", VocabularyStatus::Known),
        ("食べた", VocabularyStatus::New),
    ] {
        assert_eq!(
            storage
                .vocabulary_entry(&Surface::new(word).unwrap())
                .unwrap()
                .unwrap()
                .status,
            status
        );
    }
    let document = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap()
        .document;
    assert!(document.finished_at_least_once);
    assert_eq!(
        i64::from(document.current_position().unwrap().offset),
        imported.document.chapter.text.len() as i64
    );
    assert_eq!(
        storage
            .pending_page_time(
                document.document_id,
                frozen.completion().page,
                frozen.completion().page_length
            )
            .unwrap(),
        PendingReadingTime::default()
    );
    assert_eq!(
        storage
            .connection
            .query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        1234
    );
    let before = state(&storage);
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(state(&storage), before);
}

#[test]
fn effective_completions_register_only_departing_surfaces_and_preserve_exact_metadata() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, _) = setup(&mut storage, "食べた 食べた 猫");
    let chapter = &imported.document.chapter;
    let original = fixture(chapter).unwrap();
    let mut a = original.clone();
    a.mode = SplitMode::A;
    a.tokens = original
        .tokens
        .iter()
        .flat_map(|token| {
            if token.surface != "食べた" {
                return vec![token.clone()];
            }
            let mut first = token.clone();
            first.surface = "食べ".into();
            first.anchor.range.end = (i64::from(first.anchor.range.start) + "食べ".len() as i64)
                .try_into()
                .unwrap();
            first.reading = Some("タベ".into());
            first.dictionary_form = Some(Surface::new("食べる").unwrap());
            let mut last = token.clone();
            last.surface = "た".into();
            last.anchor.range.start = first.anchor.range.end;
            last.is_japanese = false; // exact classification excludes this effective part
            vec![first, last]
        })
        .collect();
    let a_id = storage
        .store_chapter_analysis(chapter.chapter_id, &a)
        .unwrap();
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    let frozen = prepare(&mut storage, &imported);
    assert_eq!(
        frozen.occurrences()[0]
            .metadata
            .as_ref()
            .unwrap()
            .analysis_id,
        a_id
    );
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
    );
    assert_eq!(
        surfaces(&storage),
        [
            ("食べ".into(), Some("タベ".into()), Some("食べる".into())),
            ("食べ".into(), Some("タベ".into()), Some("食べる".into())),
            ("猫".into(), None, None)
        ]
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("た").unwrap())
        .unwrap()
        .is_none());
    assert!(storage
        .vocabulary_entry(&Surface::new("食べる").unwrap())
        .unwrap()
        .is_none());
    let entry = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(entry.registered_at_unix_ms, 500);
    assert_eq!(
        storage
            .connection
            .query_row(
                "SELECT COUNT(*) FROM vocabulary_status_events WHERE vocabulary_id = ?1",
                [i64::from(entry.id)],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn effective_completions_reject_stale_forged_missing_and_foreign_selection_without_reserving() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, rule) = setup(&mut storage, "食べた 食べた");
    let (intent, key) = selection(&mut storage, &imported);
    let mut forged = key.clone();
    forged.segmentation.segments[0].boundary.surface = "別".into();
    let mut foreign = key.clone();
    foreign.document_id = i64::MAX.try_into().unwrap();
    let before = state(&storage);
    for key in [&forged, &foreign] {
        assert!(storage
            .prepare_effective_page_completion(intent, key)
            .is_err());
        assert_eq!(state(&storage), before);
    }
    storage
        .reverse_segmentation_override(rule.try_into().unwrap())
        .unwrap();
    let before = state(&storage);
    assert!(storage
        .prepare_effective_page_completion(intent, &key)
        .is_err());
    assert_eq!(state(&storage), before);
    storage
        .connection
        .execute(
            "DELETE FROM chapter_analyses WHERE id = ?1",
            [i64::from(imported.analysis_id)],
        )
        .unwrap();
    assert!(storage
        .prepare_effective_page_completion(intent, &key)
        .is_err());
    assert_eq!(rows(&storage, "completion_commands").len(), 0);
}

#[test]
fn effective_completions_reservation_survives_reversal_metadata_changes_reopen_and_deletion_replay()
{
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, rule) = setup(&mut storage, "食べた 食べた");
    let frozen = prepare(&mut storage, &imported);
    // Both APIs must reject this command, before and after it is recorded.
    for recorded in [false, true] {
        assert!(storage
            .record_analyzed_page_completion(frozen.completion(), imported.analysis_id)
            .is_err());
        assert!(storage.record_page_completion(frozen.completion()).is_err());
        if recorded {
            break;
        }
        storage
            .reverse_segmentation_override(rule.try_into().unwrap())
            .unwrap();
        let mut alternate = fixture(&imported.document.chapter).unwrap();
        alternate.mode = SplitMode::A;
        storage
            .store_chapter_analysis(imported.document.chapter.chapter_id, &alternate)
            .unwrap();
        drop(storage);
        storage = Storage::open(database.path()).unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&frozen).unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
        );
    }
    assert_eq!(
        surfaces(&storage)
            .iter()
            .map(|r| r.0.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た", "食べ", "た"]
    );
    let history = rows(&storage, "reading_occurrences");
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(imported.document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(imported.document.chapter.chapter_id)],
        )
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.record_effective_page_completion(&frozen).unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    assert_eq!(rows(&storage, "reading_occurrences"), history);
}

#[test]
fn effective_completions_every_write_and_deferred_commit_failure_roll_back_and_retry_exactly() {
    for table in [
        "reading_completions",
        "completion_analyses",
        "vocabulary",
        "vocabulary_status_events",
        "reading_occurrences",
        "reading_positions",
        "documents",
        "pending_reading_time",
        "commit",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let (imported, _) = setup(&mut storage, "食べた 食べた");
        let position = ReadingPosition {
            source_id: imported.document.chapter.source_id,
            chapter_id: imported.document.chapter.chapter_id,
            offset: TextOffset::START,
        };
        let checkpoint = storage
            .load_pending_reading_time(imported.document.document_id, position)
            .unwrap();
        storage
            .save_pending_reading_time(checkpoint, PendingReadingTime::from_millis(100).unwrap())
            .unwrap();
        let frozen = prepare(&mut storage, &imported);
        let before = state(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TABLE fault_parent(id INTEGER PRIMARY KEY); CREATE TABLE fault_child(id INTEGER REFERENCES fault_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TEMP TRIGGER fault AFTER INSERT ON reading_occurrences BEGIN INSERT INTO fault_child VALUES (1); END;").unwrap();
        } else {
            let operation = match table {
                "documents" => "UPDATE",
                "pending_reading_time" => "UPDATE",
                _ => "INSERT",
            };
            storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fault BEFORE {operation} ON {table} BEGIN SELECT RAISE(ABORT, 'failure fixture'); END;")).unwrap();
        }
        assert!(
            storage.record_effective_page_completion(&frozen).is_err(),
            "{table}"
        );
        assert_eq!(state(&storage), before, "{table}");
        storage
            .connection
            .execute_batch("DROP TRIGGER fault")
            .unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&frozen).unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 2 },
            "{table}"
        );
        assert_eq!(
            storage.record_effective_page_completion(&frozen).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    }
}

#[test]
fn effective_completions_reservation_failures_and_corrupt_payload_never_partially_register() {
    for table in ["completion_commands", "effective_completion_commands"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let (imported, _) = setup(&mut storage, "食べた");
        let (intent, key) = selection(&mut storage, &imported);
        let before = state(&storage);
        storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fault BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'failure fixture'); END;")).unwrap();
        assert!(storage
            .prepare_effective_page_completion(intent, &key)
            .is_err());
        assert_eq!(state(&storage), before);
        storage
            .connection
            .execute_batch("DROP TRIGGER fault")
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(intent, &key)
            .unwrap();
        let payload: String = storage
            .connection
            .query_row(
                "SELECT payload FROM effective_completion_commands",
                [],
                |r| r.get(0),
            )
            .unwrap();
        storage
            .connection
            .execute(
                "UPDATE effective_completion_commands SET payload = '{}'",
                [],
            )
            .unwrap();
        let before = state(&storage);
        assert!(storage.record_effective_page_completion(&frozen).is_err());
        assert_eq!(state(&storage), before);
        storage
            .connection
            .execute(
                "UPDATE effective_completion_commands SET payload = ?1",
                [&payload],
            )
            .unwrap();
        assert!(storage.record_effective_page_completion(&frozen).is_ok());
    }
}

#[test]
fn effective_completions_concurrent_duplicates_record_once_and_fresh_reread_counts_again() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, _) = setup(&mut storage, "食べた 食べた");
    let frozen = prepare(&mut storage, &imported);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let path = database.path();
            let frozen = frozen.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage.record_effective_page_completion(&frozen).unwrap()
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == AnalyzedCompletionOutcome::Recorded { newly_known: 2 })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == AnalyzedCompletionOutcome::AlreadyRecorded)
            .count(),
        1
    );
    assert_eq!(surfaces(&storage).len(), 4);
    storage
        .open_document_reader_snapshot(imported.document.document_id)
        .unwrap();
    let next = prepare(&mut storage, &imported);
    assert_ne!(next.completion().command_id, frozen.completion().command_id);
    assert_eq!(
        storage.record_effective_page_completion(&next).unwrap(),
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 }
    );
    assert_eq!(surfaces(&storage).len(), 8);
}

#[test]
fn effective_completions_all_presets_own_crossing_unicode_occurrences_once_without_registering_other_pages(
) {
    use yomoku_core::{
        analysis::{AnalyzedToken, ChapterAnalysis},
        pagination::PageLength,
    };
    for preset in PageLength::ALL {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.save_page_length(preset).unwrap();
        let tail = format!("{}終", "𠮷か\u{3099}\0。\n".repeat(850));
        let text = format!("食べた{tail}");
        let imported = storage
            .create_analyzed_paste(&PasteText::new(text).unwrap(), 100, |chapter| {
                // One synthetic Japanese B span contains sentence/paragraph boundaries
                // so the effective second part crosses every pagination preset.
                let profile = fixture(chapter).unwrap().profile;
                Ok::<_, std::convert::Infallible>(ChapterAnalysis {
                    profile,
                    mode: SplitMode::B,
                    tokens: vec![AnalyzedToken {
                        anchor: TextAnchor {
                            source_id: chapter.source_id,
                            chapter_id: chapter.chapter_id,
                            range: TextRange {
                                start: TextOffset::START,
                                end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
                            },
                        },
                        surface: chapter.text.clone(),
                        is_japanese: true,
                        reading: None,
                        dictionary_form: None,
                    }],
                })
            })
            .unwrap();
        let chapter = &imported.document.chapter;
        storage
            .save_segmentation_override(&SegmentationOverride {
                scope: OverrideScope::ThisOccurrence,
                anchor: TextAnchor {
                    source_id: chapter.source_id,
                    chapter_id: chapter.chapter_id,
                    range: TextRange {
                        start: TextOffset::START,
                        end: TextOffset::at(&chapter.text, chapter.text.len()).unwrap(),
                    },
                },
                surface: Surface::new(&chapter.text).unwrap(),
                parts: vec![
                    Surface::new("食べた").unwrap(),
                    Surface::new(&tail).unwrap(),
                ],
            })
            .unwrap();
        let pages = LogicalPages::new(std::slice::from_ref(chapter), preset).unwrap();
        assert!(pages.pages().len() > 1);
        for (index, _) in pages.pages().iter().enumerate() {
            let frozen = prepare(&mut storage, &imported);
            assert_eq!(frozen.occurrences().len(), if index == 0 { 2 } else { 0 });
            let outcome = storage.record_effective_page_completion(&frozen).unwrap();
            assert_eq!(
                outcome,
                AnalyzedCompletionOutcome::Recorded {
                    newly_known: if index == 0 { 2 } else { 0 }
                }
            );
        }
        assert_eq!(
            surfaces(&storage),
            [("食べた".into(), None, None), (tail, None, None)]
        );
    }
    // A real later-page spelling deleted from vocabulary is never recreated by
    // an earlier completion, including after the whole chapter was validated.
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, _) = setup(
        &mut storage,
        &format!("食べた\n{}\n猫", "English ".repeat(400)),
    );
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    let frozen = prepare(&mut storage, &imported);
    storage.record_effective_page_completion(&frozen).unwrap();
    assert!(storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn effective_completions_migration_preserves_old_history_and_failed_upgrade_rolls_back() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let (imported, _) = setup(&mut storage, "食べた");
    let (intent, _) = selection(&mut storage, &imported);
    let old = storage.prepare_page_completion(intent).unwrap();
    storage
        .record_analyzed_page_completion(old.completion, old.analysis_id)
        .unwrap();
    // Reconstruct the exact v17 schema; new table is still empty.
    storage.connection.execute_batch("DROP TABLE effective_completion_commands; DROP TABLE library_read_clock; ALTER TABLE documents DROP COLUMN last_read_sequence; DELETE FROM schema_migrations WHERE version >= 18;").unwrap();
    let before = rows(&storage, "reading_occurrences");
    storage
        .connection
        .execute_batch("CREATE VIEW effective_completion_commands AS SELECT 1;")
        .unwrap();
    drop(storage);
    assert!(Storage::open(database.path()).is_err());
    let connection = Connection::open(database.path()).unwrap();
    assert_eq!(history(&database.path()).last().unwrap().0, 17);
    connection
        .execute_batch("DROP VIEW effective_completion_commands")
        .unwrap();
    drop(connection);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 19);
    assert_eq!(rows(&storage, "reading_occurrences"), before);
    assert!(rows(&storage, "effective_completion_commands").is_empty());
    assert_eq!(
        storage
            .record_analyzed_page_completion(old.completion, old.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
}
