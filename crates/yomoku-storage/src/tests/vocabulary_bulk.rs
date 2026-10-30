use rusqlite::types::Value;
use yomoku_core::{
    active_time::PendingReadingTime,
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{PasteText, VocabularyId},
    reader::ReaderPage,
    statistics::StatisticsTimeframe,
    vocabulary::{Surface, VocabularyQuery, VocabularySort, VocabularyStatus},
};

use super::*;

fn register(storage: &mut Storage, text: &str, date: i64) -> VocabularyId {
    let surface = Surface::new(text).unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&surface), date)
        .unwrap();
    storage.vocabulary_entry(&surface).unwrap().unwrap().id
}

fn table(storage: &Storage, name: &str) -> Vec<Vec<Value>> {
    let mut statement = storage
        .connection
        .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| {
            (0..columns).map(|column| row.get(column)).collect()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn history(storage: &Storage) -> Vec<Vec<Vec<Value>>> {
    [
        "vocabulary",
        "vocabulary_status_events",
        "vocabulary_dictionary_forms",
        "reading_occurrences",
        "reading_completions",
        "completion_analyses",
        "effective_completion_commands",
        "sqlite_sequence",
    ]
    .map(|name| table(storage, name))
    .to_vec()
}

fn import(storage: &mut Storage, text: &str, date: i64) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), date, |chapter| {
            let mut analysis = imports::fixture(chapter).unwrap();
            for token in &mut analysis.tokens {
                if token.is_japanese {
                    token.dictionary_form = Some(Surface::new(&token.surface).unwrap());
                }
            }
            Ok::<_, std::convert::Infallible>(analysis)
        })
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

#[test]
fn matching_ids_include_unloaded_rows_and_share_literal_prefix_status_set_semantics() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let statuses = VocabularyQuery::default().statuses;
    let mut input = Vec::new();
    for index in 0..1205 {
        let text = format!("猫{index:04}");
        let id = register(&mut storage, &text, 100);
        let status = statuses[index % 5];
        storage.set_vocabulary_status(id, status, 200).unwrap();
        input.push((id, text, status));
    }
    for text in [
        "%語",
        "_語",
        "語\0尾",
        "語\0外",
        "𠮷",
        "が",
        "か\u{3099}",
        " 猫",
        "A",
        "a",
        "\u{10ffff}語",
    ] {
        let id = register(&mut storage, text, 100);
        input.push((id, text.into(), VocabularyStatus::New));
    }
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for mask in 0..32 {
        for prefix in [
            "",
            "猫",
            "%",
            "_",
            "語\0",
            "𠮷",
            "が",
            "か",
            " ",
            "A",
            "a",
            "\u{10ffff}",
            "不存在",
        ] {
            for sort in [
                VocabularySort::Newest,
                VocabularySort::Frequency,
                VocabularySort::LastSeen,
                VocabularySort::Alphabetical,
            ] {
                let mut query = VocabularyQuery {
                    prefix: prefix.into(),
                    statuses: statuses
                        .iter()
                        .enumerate()
                        .filter_map(|(index, status)| (mask & (1 << index) != 0).then_some(*status))
                        .collect(),
                    sort,
                };
                // Duplicates and order must not change set membership.
                query.statuses.extend(query.statuses.clone());
                query.statuses.reverse();
                let expected: Vec<_> = input
                    .iter()
                    .filter(|(_, surface, status)| {
                        surface.starts_with(prefix) && query.statuses.contains(status)
                    })
                    .map(|(id, _, _)| *id)
                    .collect();
                let ids = storage.vocabulary_matching_ids(&query).unwrap();
                assert_eq!(ids, expected, "{query:?}");
                let page = storage.vocabulary_page(&query, 0).unwrap();
                assert_eq!(page.matching_count, ids.len() as u64);
                assert!(page.rows.iter().all(|row| ids.contains(&row.id)));
            }
        }
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    drop(storage);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .vocabulary_matching_ids(&VocabularyQuery::default())
            .unwrap()
            .len(),
        input.len()
    );
}

#[test]
fn bulk_statuses_append_only_actual_transitions_for_all_statuses_and_large_selections() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input: Vec<_> = (0..1205)
        .map(|index| Surface::new(format!("語{index}")).unwrap())
        .collect();
    storage.register_vocabulary(&input, 100).unwrap();
    let mut ids = storage
        .vocabulary_matching_ids(&VocabularyQuery::default())
        .unwrap();
    ids.extend(ids.clone());
    ids.push(VocabularyId::try_from(i64::MAX).unwrap());
    let mut transitions = 0;
    for (index, status) in [
        VocabularyStatus::New,
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ]
    .into_iter()
    .enumerate()
    {
        let before = table(&storage, "vocabulary_status_events");
        let date = 200 + index as i64;
        storage.set_vocabulary_statuses(&ids, status, date).unwrap();
        let after = table(&storage, "vocabulary_status_events");
        assert_eq!(&after[..before.len()], before);
        if index > 0 {
            transitions += 1;
        }
        assert_eq!(after.len(), 1205 * (transitions + 1));
        if index > 0 {
            for event in &after[before.len()..] {
                assert_eq!(event[1], Value::Integer(1));
                assert_eq!(event[5], Value::Text("manual".into()));
                assert_eq!(event[6], Value::Integer(date));
                assert_eq!(event[7], Value::Null);
            }
        }
        storage.set_vocabulary_statuses(&ids, status, date).unwrap();
        assert_eq!(table(&storage, "vocabulary_status_events"), after);
        assert!(storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap()
            .rows
            .iter()
            .all(|row| row.status == status
                && row.registered_at_unix_ms == 100
                && row.encountered_count == 0));
        let counts = storage
            .vocabulary_status_timeline()
            .unwrap()
            .last()
            .unwrap()
            .counts;
        assert_eq!(
            counts.new + counts.learning + counts.familiar + counts.known + counts.ignored,
            1205
        );
    }
    let before = history(&storage);
    assert!(matches!(
        storage.set_vocabulary_statuses(&ids, VocabularyStatus::Known, -1),
        Err(StorageError::InvalidVocabulary)
    ));
    assert!(matches!(
        storage.set_vocabulary_statuses(&[], VocabularyStatus::Known, -1),
        Err(StorageError::InvalidVocabulary)
    ));
    storage
        .set_vocabulary_statuses(&[], VocabularyStatus::Known, 300)
        .unwrap();
    assert_eq!(history(&storage), before);
    storage.delete_vocabulary_entries(&ids).unwrap();
    assert!(storage
        .vocabulary_matching_ids(&VocabularyQuery::default())
        .unwrap()
        .is_empty());
    assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
}

#[test]
fn frozen_selection_does_not_requery_or_target_new_spelling_generations() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = register(&mut storage, "猫", 100);
    let second = register(&mut storage, "猫舌", 100);
    let kept = register(&mut storage, "犬", 100);
    let query = VocabularyQuery {
        prefix: "猫".into(),
        statuses: vec![VocabularyStatus::New],
        ..Default::default()
    };
    let ids = storage.vocabulary_matching_ids(&query).unwrap();
    assert_eq!(ids, [first, second]);
    let mut other = Storage::open(database.path()).unwrap();
    other.delete_vocabulary_entries(&[first]).unwrap();
    let fresh = register(&mut other, "猫", 300);
    let later = register(&mut other, "猫背", 300);
    other
        .set_vocabulary_status(second, VocabularyStatus::Ignored, 300)
        .unwrap();
    drop(other);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .set_vocabulary_statuses(&ids, VocabularyStatus::Known, 400)
        .unwrap();
    let page = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    assert!(page
        .rows
        .iter()
        .filter(|row| [fresh, later, kept].contains(&row.id))
        .all(|row| row.status == VocabularyStatus::New));
    assert_eq!(
        page.rows
            .iter()
            .find(|row| row.id == second)
            .unwrap()
            .status,
        VocabularyStatus::Known
    );
    storage.delete_vocabulary_entries(&ids).unwrap();
    let after = history(&storage);
    storage.delete_vocabulary_entries(&ids).unwrap();
    storage.delete_vocabulary_entries(&[]).unwrap();
    storage
        .set_vocabulary_statuses(&ids, VocabularyStatus::Learning, 500)
        .unwrap();
    assert_eq!(history(&storage), after);
    assert_eq!(
        storage
            .vocabulary_matching_ids(&VocabularyQuery::default())
            .unwrap(),
        [kept, fresh, later]
    );
}

#[test]
fn later_status_statement_event_and_commit_failures_roll_back_every_row_and_retry_exactly() {
    for sql in [
        "CREATE TRIGGER fail BEFORE UPDATE ON vocabulary WHEN OLD.surface = '犬' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE INSERT ON vocabulary_status_events WHEN NEW.vocabulary_id = 2 AND NEW.cause = 'manual' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER INSERT ON vocabulary_status_events WHEN NEW.vocabulary_id = 2 BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let ids = [register(&mut storage, "猫", 100), register(&mut storage, "犬", 100)];
        storage.connection.execute_batch(sql).unwrap();
        let before = history(&storage);
        assert!(matches!(storage.set_vocabulary_statuses(&ids, VocabularyStatus::Known, 200), Err(StorageError::Sqlite(_))));
        assert_eq!(history(&storage), before);
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(history(&storage), before);
        let later = register(&mut storage, "𠮷", 300);
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        storage.set_vocabulary_statuses(&ids, VocabularyStatus::Known, 200).unwrap();
        let timeline = storage.vocabulary_status_timeline().unwrap();
        assert_eq!(timeline.last().unwrap().counts.known, 2);
        assert_eq!(timeline.last().unwrap().counts.new, 1);
        assert_eq!(storage.vocabulary_matching_ids(&VocabularyQuery { statuses: vec![VocabularyStatus::New], ..Default::default() }).unwrap(), [later]);
    }
}

#[test]
fn later_delete_cascade_and_commit_failures_preserve_history_charts_and_exact_retry() {
    for sql in [
        "CREATE TRIGGER fail BEFORE DELETE ON vocabulary WHEN OLD.surface = '犬' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON reading_occurrences WHEN OLD.vocabulary_id = 2 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON vocabulary_status_events WHEN OLD.vocabulary_id = 2 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON vocabulary_dictionary_forms WHEN OLD.vocabulary_id = 2 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER DELETE ON vocabulary WHEN OLD.surface = '犬' BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫 猫 犬", 100);
        let intent = intent(&mut storage, &imported);
        let command = storage.prepare_page_completion(intent).unwrap();
        storage.record_analyzed_page_completion(command.completion, command.analysis_id).unwrap();
        let ids = storage.vocabulary_matching_ids(&VocabularyQuery::default()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        let before = history(&storage);
        let charts = storage.statistics_buckets(StatisticsTimeframe::AllTime, 1000).unwrap();
        assert!(matches!(storage.delete_vocabulary_entries(&ids), Err(StorageError::Sqlite(_))));
        assert_eq!(history(&storage), before);
        assert_eq!(storage.statistics_buckets(StatisticsTimeframe::AllTime, 1000).unwrap(), charts);
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(history(&storage), before);
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        storage.delete_vocabulary_entries(&ids).unwrap();
        assert!(storage.vocabulary_status_timeline().unwrap().is_empty());
        assert_eq!(storage.lifetime_reading_totals().unwrap().japanese_occurrences, 0);
        assert_eq!(table(&storage, "reading_completions").len(), 1);
    }
}

#[test]
fn deletion_recalculates_original_effective_charts_without_rescan_and_new_appearance_is_new() {
    for effective in [false, true] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = import(&mut storage, "猫 猫 犬", 100);
        let deleted_document = import(&mut storage, "猫 𠮷", 100);
        let intent = intent(&mut storage, &live);
        let pending = storage
            .load_pending_reading_time(intent.document_id, intent.departing_position)
            .unwrap();
        storage
            .save_pending_reading_time(pending, PendingReadingTime::from_millis(321).unwrap())
            .unwrap();
        let original = storage.prepare_page_completion(intent).unwrap();
        let key = storage
            .load_effective_reader_tokens(intent.document_id, intent.page)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(intent, &key)
            .unwrap();
        let complete = |storage: &mut Storage| {
            if effective {
                storage.record_effective_page_completion(&frozen)
            } else {
                storage.record_analyzed_page_completion(original.completion, original.analysis_id)
            }
        };
        assert_eq!(
            complete(&mut storage).unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
        );
        storage
            .delete_document(deleted_document.document.document_id)
            .unwrap();
        let before_totals = storage.lifetime_reading_totals().unwrap();
        assert_eq!(before_totals.japanese_occurrences, 3);
        assert_eq!(before_totals.active_time_ms, 321);
        let cat = storage
            .vocabulary_matching_ids(&VocabularyQuery {
                prefix: "猫".into(),
                ..Default::default()
            })
            .unwrap();
        let preserved: Vec<_> = [
            "documents",
            "sources",
            "chapters",
            "chapter_texts",
            "chapter_analyses",
            "analyzed_tokens",
            "reading_positions",
            "reading_completions",
            "completion_analyses",
            "effective_completion_commands",
        ]
        .map(|name| table(&storage, name))
        .to_vec();
        storage.delete_vocabulary_entries(&cat).unwrap();
        assert_eq!(
            preserved,
            [
                "documents",
                "sources",
                "chapters",
                "chapter_texts",
                "chapter_analyses",
                "analyzed_tokens",
                "reading_positions",
                "reading_completions",
                "completion_analyses",
                "effective_completion_commands"
            ]
            .map(|name| table(&storage, name))
            .to_vec()
        );
        assert_eq!(
            complete(&mut storage).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
        let totals = storage.lifetime_reading_totals().unwrap();
        assert_eq!(totals.japanese_occurrences, 1);
        assert_eq!(totals.active_time_ms, 321);
        let timeline = storage.vocabulary_status_timeline().unwrap();
        assert_eq!(timeline[0].counts.new, 2);
        assert_eq!(timeline.last().unwrap().counts.known, 1);
        assert_eq!(timeline.last().unwrap().known_dictionary_forms, 1);
        for frame in [
            StatisticsTimeframe::LastMonth,
            StatisticsTimeframe::LastSixMonths,
            StatisticsTimeframe::LastYear,
            StatisticsTimeframe::AllTime,
        ] {
            let chart = storage.statistics_buckets(frame, 1000).unwrap();
            let last = chart.buckets.last().unwrap().values;
            assert_eq!(last.reading, totals);
            assert_eq!(last.statuses, timeline.last().unwrap().counts);
            assert_eq!(last.known_dictionary_forms, 1);
        }
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        storage
            .open_document_reader_snapshot(live.document.document_id)
            .unwrap();
        storage
            .load_document_reader_snapshot(live.document.document_id)
            .unwrap();
        storage
            .load_effective_reader_tokens(intent.document_id, intent.page)
            .unwrap();
        storage.effective_library_word_distributions().unwrap();
        storage.library_word_distributions().unwrap();
        assert_eq!(
            storage
                .vocabulary_entry(&Surface::new("猫").unwrap())
                .unwrap(),
            None
        );
        assert_eq!(storage.lifetime_reading_totals().unwrap(), totals);
        let fresh_document = import(&mut storage, "猫 猫", 800);
        let fresh = storage
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap();
        assert!(fresh.id > cat[0]);
        assert_eq!(fresh.status, VocabularyStatus::New);
        assert_eq!(fresh.registered_at_unix_ms, 800);
        storage.delete_vocabulary_entries(&cat).unwrap();
        storage
            .set_vocabulary_statuses(&cat, VocabularyStatus::Ignored, 900)
            .unwrap();
        assert_eq!(
            storage.vocabulary_entry(&fresh.surface).unwrap(),
            Some(fresh)
        );
        let rows = storage
            .vocabulary_page(
                &VocabularyQuery {
                    prefix: "猫".into(),
                    ..Default::default()
                },
                0,
            )
            .unwrap()
            .rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].encountered_count, 0);
        assert_eq!(rows[0].last_seen_unix_ms, None);
        let mut new_intent = intent;
        new_intent.completed_at_unix_ms = 1000;
        let new_command = storage.prepare_page_completion(new_intent).unwrap();
        storage
            .record_analyzed_page_completion(new_command.completion, new_command.analysis_id)
            .unwrap();
        assert_eq!(
            storage
                .lifetime_reading_totals()
                .unwrap()
                .japanese_occurrences,
            4
        );
        assert!(storage
            .load_document_reader_snapshot(fresh_document.document.document_id)
            .unwrap()
            .is_some());
    }
}

#[test]
fn concurrent_bulk_status_retries_append_one_transition_per_entry() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let ids = [
        register(&mut storage, "猫", 100),
        register(&mut storage, "犬", 100),
    ];
    let barrier = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = database.path();
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    let mut storage = Storage::open(path).unwrap();
                    barrier.wait();
                    storage
                        .set_vocabulary_statuses(&ids, VocabularyStatus::Known, 200)
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    assert_eq!(table(&storage, "vocabulary_status_events").len(), 4);
    assert_eq!(
        storage
            .vocabulary_status_timeline()
            .unwrap()
            .last()
            .unwrap()
            .counts
            .known,
        2
    );
}
