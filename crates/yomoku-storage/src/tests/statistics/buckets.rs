use yomoku_core::statistics::{
    StatisticsBuckets,
    StatisticsTimeframe::{self, *},
};

use super::*;

const FEB_28: i64 = 1_709_078_400_000;
const FEB_29: i64 = 1_709_164_800_000;
const MAR_31: i64 = 1_711_843_200_000;

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, |chapter| {
            let mut analysis = fixture(chapter).unwrap();
            for token in &mut analysis.tokens {
                if token.is_japanese {
                    token.dictionary_form = Some(Surface::new(&token.surface).unwrap());
                }
            }
            Ok::<_, std::convert::Infallible>(analysis)
        })
        .unwrap()
}

fn complete_at(storage: &mut Storage, imported: &AnalyzedPaste, timestamp: i64, effective: bool) {
    let mut intent = intent(storage, imported);
    intent.completed_at_unix_ms = timestamp;
    if effective {
        let key = storage
            .load_effective_reader_tokens(intent.document_id, intent.page)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let command = storage
            .prepare_effective_page_completion(intent, &key)
            .unwrap();
        storage.record_effective_page_completion(&command).unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&command).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    } else {
        let command = storage.prepare_page_completion(intent).unwrap();
        storage
            .record_analyzed_page_completion(command.completion, command.analysis_id)
            .unwrap();
        assert_eq!(
            storage
                .record_analyzed_page_completion(command.completion, command.analysis_id)
                .unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    }
    storage
        .save_document_reading_position(imported.document.document_id, intent.departing_position)
        .unwrap();
}

fn buckets(storage: &mut Storage, frame: StatisticsTimeframe) -> StatisticsBuckets {
    storage.statistics_buckets(frame, MAR_31).unwrap()
}

#[test]
fn buckets_keep_original_effective_rereads_lifetime_origin_and_all_lines_read_only() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    save_time(&mut storage, &imported, 111);
    complete_at(&mut storage, &imported, FEB_28, false);
    save_time(&mut storage, &imported, 222);
    complete_at(&mut storage, &imported, FEB_29, true);
    // Another analyzed command on the exact same date must add time once, even
    // with no Japanese occurrences. It must not disappear in an inner join.
    let english = import(&mut storage, "English 。 📖");
    save_time(&mut storage, &english, 333);
    complete_at(&mut storage, &english, FEB_29, true);
    save_time(&mut storage, &imported, 444);
    complete_at(&mut storage, &imported, MAR_31, true);
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let lifetime = storage.lifetime_reading_totals().unwrap();
    assert_eq!(
        lifetime,
        LifetimeReadingTotals {
            japanese_occurrences: 9,
            active_time_ms: 1110
        }
    );
    let status = storage.vocabulary_status_timeline().unwrap();
    for frame in [LastMonth, LastSixMonths, LastYear, AllTime] {
        let result = buckets(&mut storage, frame);
        let last = result.buckets.last().unwrap().values;
        assert_eq!(last.reading, lifetime);
        assert_eq!(last.statuses, status.last().unwrap().counts);
        assert_eq!(last.statuses.known, 2);

        if frame == LastMonth {
            assert_eq!(result.start_unix_ms, FEB_29);
            assert_eq!(
                result.opening.reading,
                LifetimeReadingTotals {
                    japanese_occurrences: 3,
                    active_time_ms: 111
                }
            );
            assert_eq!(
                result.buckets[0].values.reading,
                LifetimeReadingTotals {
                    japanese_occurrences: 6,
                    active_time_ms: 666
                }
            );
        }
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        buckets(&mut storage, LastMonth)
            .buckets
            .last()
            .unwrap()
            .values
            .reading,
        lifetime
    );
}

#[test]
fn imports_pending_time_unused_reservations_and_legacy_history_add_no_reading() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(buckets(&mut storage, AllTime).buckets.len(), 1);
    let imported = import(&mut storage, "猫 猫 犬");
    save_time(&mut storage, &imported, 999);
    prepare(&mut storage, &imported);
    let legacy = import(&mut storage, "𠮷");
    save_time(&mut storage, &legacy, 1000);
    let command_intent = intent(&mut storage, &legacy);
    let command = storage.prepare_page_completion(command_intent).unwrap();
    storage.record_page_completion(command.completion).unwrap();
    let result = buckets(&mut storage, LastMonth);
    assert_eq!(result.opening.statuses.new, 3);
    assert!(result
        .buckets
        .iter()
        .all(|b| b.values.reading == LifetimeReadingTotals::default()));
    assert!(result.buckets.iter().all(|b| b.values.statuses.new == 3));
}

#[test]
fn deletions_recalculate_buckets_without_rescanning_and_fresh_generations_start_new() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    save_time(&mut storage, &imported, 1234);
    complete_at(&mut storage, &imported, FEB_28, true);
    let before = buckets(&mut storage, LastMonth);
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    assert_eq!(buckets(&mut storage, LastMonth), before);
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?1", [i64::from(cat.id)])
        .unwrap();
    let erased = buckets(&mut storage, LastMonth);
    assert_eq!(
        erased.opening.reading,
        LifetimeReadingTotals {
            japanese_occurrences: 1,
            active_time_ms: 1234
        }
    );
    assert_eq!(erased.opening.statuses.known, 1);

    let reimported = storage
        .create_analyzed_paste(&PasteText::new("猫 猫 犬".into()).unwrap(), MAR_31, fixture)
        .unwrap();
    let fresh = buckets(&mut storage, LastMonth);
    assert_eq!(fresh.opening, erased.opening);
    let last = fresh.buckets.last().unwrap().values;
    assert_eq!(last.reading, erased.opening.reading);
    assert_eq!(last.statuses.new, 1);
    assert_eq!(last.statuses.known, 1);

    complete_at(&mut storage, &reimported, MAR_31, true);
    assert_eq!(
        buckets(&mut storage, LastMonth)
            .buckets
            .last()
            .unwrap()
            .values
            .reading
            .japanese_occurrences,
        4
    );
}

#[test]
fn failed_and_uncommitted_work_is_invisible_and_the_cutoff_excludes_future_completions() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let pending = prepare(&mut storage, &imported);
    let before = buckets(&mut storage, LastMonth);
    storage.connection.execute_batch("CREATE TRIGGER fail BEFORE INSERT ON reading_occurrences WHEN NEW.token_index = 1 BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage.record_effective_page_completion(&pending).is_err());
    assert_eq!(buckets(&mut storage, LastMonth), before);
    storage
        .connection
        .execute_batch("DROP TRIGGER fail")
        .unwrap();
    storage.record_effective_page_completion(&pending).unwrap();
    let expected = buckets(&mut storage, LastMonth);
    let mut writer = Storage::open(database.path()).unwrap();
    let transaction = writer.connection.transaction().unwrap();
    transaction
        .execute(
            "UPDATE reading_completions SET completed_at_unix_ms = ?1",
            [MAR_31 + 1],
        )
        .unwrap();
    transaction.execute("UPDATE vocabulary_status_events SET occurred_at_unix_ms = ?1 WHERE cause = 'completion'", [MAR_31 + 1]).unwrap();
    assert_eq!(buckets(&mut storage, LastMonth), expected);
    transaction.commit().unwrap();
    let future = buckets(&mut storage, LastMonth);
    assert_eq!(
        future.buckets.last().unwrap().values.reading,
        LifetimeReadingTotals::default()
    );
    assert_eq!(future.buckets.last().unwrap().values.statuses.new, 2);
    assert_eq!(future.buckets.last().unwrap().values.statuses.known, 0);
}

#[test]
fn invalid_versions_dates_and_time_fail_without_repair_or_leaving_a_transaction_open() {
    for sql in [
        "UPDATE reading_completions SET semantics_version = 2",
        "UPDATE completion_analyses SET semantics_version = 2",
        "UPDATE effective_completion_commands SET semantics_version = 3",
        "UPDATE reading_completions SET active_time_ms = -1",
        "UPDATE reading_completions SET completed_at_unix_ms = -1",
        "UPDATE reading_completions SET completed_at_unix_ms = 9223372036854775807",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        // No completion-caused status transition: exercise reading validation.
        let imported = import(&mut storage, "English");
        complete_at(&mut storage, &imported, FEB_28, true);
        storage
            .connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        storage.connection.execute(sql, []).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(
            storage.statistics_buckets(LastYear, MAR_31).is_err(),
            "{sql}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert!(storage.connection.is_autocommit());
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(matches!(
        storage.statistics_buckets(LastYear, -1),
        Err(StorageError::InvalidStatistics)
    ));
    assert!(storage.connection.is_autocommit());
}

#[test]
fn same_date_integer_sum_overflow_fails_without_writing() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    for (text, duration) in [("猫", i64::MAX), ("犬", 1)] {
        let imported = import(&mut storage, text);
        save_time(&mut storage, &imported, duration);
        complete_at(&mut storage, &imported, FEB_28, true);
    }
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        storage.statistics_buckets(AllTime, MAR_31),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert!(storage.connection.is_autocommit());
}

#[test]
fn cross_date_totals_keep_the_lifetime_signed_integer_limit() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    for (text, duration, date) in [("猫", i64::MAX, FEB_28), ("犬", 1, FEB_29)] {
        let imported = import(&mut storage, text);
        save_time(&mut storage, &imported, duration);
        complete_at(&mut storage, &imported, date, true);
    }
    let before = fs::read(database.path()).unwrap();
    assert!(storage.lifetime_reading_totals().is_err());
    assert!(matches!(
        storage.statistics_buckets(AllTime, MAR_31),
        Err(StorageError::InvalidCompletion)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert!(storage.connection.is_autocommit());
}
