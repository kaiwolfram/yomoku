use rusqlite::{params_from_iter, types::Value};
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{PasteText, VocabularyId},
    reader::ReaderPage,
    vocabulary::{Surface, VocabularyQuery, VocabularyRow, VocabularySort, VocabularyStatus},
};

use super::{imports::fixture, *};

fn register(storage: &mut Storage, text: &str, date: i64) -> VocabularyId {
    let surface = Surface::new(text).unwrap();
    storage
        .register_vocabulary(std::slice::from_ref(&surface), date)
        .unwrap();
    storage.vocabulary_entry(&surface).unwrap().unwrap().id
}

fn query(prefix: &str, sort: VocabularySort) -> VocabularyQuery {
    VocabularyQuery {
        prefix: prefix.into(),
        sort,
        ..Default::default()
    }
}

fn rows(storage: &Storage, query: &VocabularyQuery) -> Vec<VocabularyRow> {
    storage.vocabulary_page(query, 0).unwrap().rows
}

fn surfaces(rows: &[VocabularyRow]) -> Vec<&str> {
    rows.iter().map(|row| row.surface.as_str()).collect()
}

fn import(storage: &mut Storage, text: &str) -> AnalyzedPaste {
    storage
        .create_analyzed_paste(&PasteText::new(text.into()).unwrap(), 100, fixture)
        .unwrap()
}

fn intent(storage: &mut Storage, imported: &AnalyzedPaste, date: i64) -> PageCompletionIntent {
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
        completed_at_unix_ms: date,
        analysis_id: imported.analysis_id,
    }
}

fn complete(storage: &mut Storage, imported: &AnalyzedPaste, date: i64, effective: bool) {
    let intent = intent(storage, imported, date);
    if effective {
        let key = storage
            .load_effective_reader_tokens(intent.document_id, intent.page)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(intent, &key)
            .unwrap();
        storage.record_effective_page_completion(&frozen).unwrap();
        assert_eq!(
            storage.record_effective_page_completion(&frozen).unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    } else {
        let frozen = storage.prepare_page_completion(intent).unwrap();
        storage
            .record_analyzed_page_completion(frozen.completion, frozen.analysis_id)
            .unwrap();
        assert_eq!(
            storage
                .record_analyzed_page_completion(frozen.completion, frozen.analysis_id)
                .unwrap(),
            AnalyzedCompletionOutcome::AlreadyRecorded
        );
    }
    storage
        .save_document_reading_position(imported.document.document_id, intent.departing_position)
        .unwrap();
}

#[test]
fn literal_prefixes_preserve_unicode_case_nul_whitespace_and_wildcard_characters() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let input = [
        "猫",
        "猫舌",
        "山猫",
        "食べた",
        "食べる",
        "が",
        "か\u{3099}",
        "𠮷",
        "𠮷野",
        "A",
        "a",
        "%語",
        "_語",
        "\\語",
        "語\0尾",
        "語\0先",
        "語外",
        " 猫",
        "猫 ",
        "' OR 1=1 --",
        "\u{d7ff}語",
        "\u{e000}語",
        "\u{10ffff}",
        "\u{10ffff}語",
        "\u{10ffff}\u{10ffff}語",
        "猫\u{10ffff}語",
        "猫\u{10ffff}\u{10ffff}",
    ];
    for text in input {
        register(&mut storage, text, 100);
    }
    for prefix in [
        "",
        "猫",
        "舌",
        "食べ",
        "食べた",
        "が",
        "か",
        "𠮷",
        "A",
        "a",
        "%",
        "_",
        "\\",
        "語\0",
        "語\0尾",
        " ",
        "猫 ",
        "' OR 1=1 --",
        "\u{d7ff}",
        "\u{10ffff}",
        "\u{10ffff}\u{10ffff}",
        "猫\u{10ffff}",
        "\0",
        "不存在",
    ] {
        let mut expected: Vec<_> = input
            .into_iter()
            .filter(|surface| surface.starts_with(prefix))
            .collect();
        expected.sort();
        let page = storage
            .vocabulary_page(&query(prefix, VocabularySort::Alphabetical), 0)
            .unwrap();
        assert_eq!(surfaces(&page.rows), expected, "prefix {prefix:?}");
        assert_eq!(page.matching_count, expected.len() as u64);
        assert!(page.next_offset.is_none());
    }
}

#[test]
fn prefix_never_matches_reading_or_dictionary_form_and_lookup_never_registers() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .create_analyzed_paste(&PasteText::new("食べた".into()).unwrap(), 1, |chapter| {
            let mut analysis = fixture(chapter).unwrap();
            analysis.tokens[0].reading = Some("タベタ".into());
            analysis.tokens[0].dictionary_form = Some(Surface::new("食べる").unwrap());
            Ok::<_, std::convert::Infallible>(analysis)
        })
        .unwrap();
    for prefix in ["タベ", "たべ", "食べる", "べた"] {
        assert!(rows(&storage, &query(prefix, VocabularySort::Newest)).is_empty());
    }
    assert_eq!(
        surfaces(&rows(&storage, &VocabularyQuery::default())),
        ["食べた"]
    );
    assert!(storage
        .vocabulary_entry(&Surface::new("食べる").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn all_status_subsets_intersect_prefix_and_empty_and_duplicate_filters_are_explicit() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let statuses = VocabularyQuery::default().statuses;
    for (index, status) in statuses.iter().enumerate() {
        let id = register(&mut storage, &format!("語{index}"), 100);
        storage.set_vocabulary_status(id, *status, 200).unwrap();
        let id = register(&mut storage, &format!("別{index}"), 100);
        storage.set_vocabulary_status(id, *status, 200).unwrap();
    }
    for mask in 0..32 {
        let selected: Vec<_> = statuses
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, status)| *status)
            .collect();
        let mut query = query("語", VocabularySort::Alphabetical);
        query.statuses = selected.iter().chain(&selected).copied().collect();
        let page = storage.vocabulary_page(&query, 0).unwrap();
        assert_eq!(page.matching_count, selected.len() as u64);
        assert_eq!(page.rows.len(), selected.len());
        assert!(page.rows.iter().all(|row| selected.contains(&row.status)));
    }
}

#[test]
fn hundred_row_pages_keep_exact_counts_stable_ties_and_no_gaps_for_every_sort() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut expected = Vec::new();
    for index in 0..257 {
        let surface = format!("語{:03}", 256 - index);
        let date = index % 3;
        let id = register(&mut storage, &surface, date);
        let status = if index % 5 == 0 {
            VocabularyStatus::Known
        } else {
            VocabularyStatus::New
        };
        storage.set_vocabulary_status(id, status, 10).unwrap();
        expected.push((id, surface, date, status));
    }
    register(&mut storage, "別", 100);
    for sort in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
    ] {
        for filter in [
            vec![VocabularyStatus::New],
            VocabularyQuery::default().statuses,
        ] {
            let mut query = query("語", sort);
            query.statuses = filter.clone();
            let mut expected: Vec<_> = expected
                .iter()
                .filter(|row| filter.contains(&row.3))
                .collect();
            expected.sort_by(|a, b| match sort {
                VocabularySort::Alphabetical => a.1.cmp(&b.1),
                _ => b.2.cmp(&a.2).then_with(|| b.0.cmp(&a.0)),
            });
            let mut offset = 0;
            let mut actual = Vec::new();
            loop {
                let page = storage.vocabulary_page(&query, offset).unwrap();
                assert_eq!(page.offset, offset);
                assert_eq!(page.matching_count, expected.len() as u64);
                assert_eq!(page.rows.len(), 100.min(expected.len() - offset as usize));
                assert!(page
                    .rows
                    .iter()
                    .all(|row| row.encountered_count == 0 && row.last_seen_unix_ms.is_none()));
                actual.extend(page.rows.into_iter().map(|row| row.id));
                let Some(next) = page.next_offset else {
                    break;
                };
                assert_eq!(next, offset + 100);
                offset = next;
            }
            assert_eq!(actual, expected.iter().map(|row| row.0).collect::<Vec<_>>());
            for offset in [
                expected.len() as u64,
                expected.len() as u64 + 100,
                i64::MAX as u64,
            ] {
                let page = storage.vocabulary_page(&query, offset).unwrap();
                assert_eq!(page.matching_count, expected.len() as u64);
                assert!(page.rows.is_empty());
                assert!(page.next_offset.is_none());
            }
        }
    }
    assert!(matches!(
        storage.vocabulary_page(&VocabularyQuery::default(), u64::MAX),
        Err(StorageError::InvalidVocabulary)
    ));
}

#[test]
fn counts_and_last_seen_use_original_effective_repetitions_rereads_and_latest_wall_date() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cat = import(&mut storage, "猫 猫 食べた English 。 📖");
    let food = import(&mut storage, "食べた");
    let dog = import(&mut storage, "犬");
    register(&mut storage, "未読", 999);
    complete(&mut storage, &cat, 500, false);
    complete(&mut storage, &cat, 300, true);
    complete(&mut storage, &food, 800, true);
    complete(&mut storage, &dog, 900, false);
    let cat_id = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap()
        .id;
    storage
        .set_vocabulary_status(cat_id, VocabularyStatus::Learning, 9999)
        .unwrap();
    assert!(storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .is_some());
    let by_frequency = rows(&storage, &query("", VocabularySort::Frequency));
    assert_eq!(surfaces(&by_frequency), ["猫", "食べた", "犬", "未読"]);
    assert_eq!(
        by_frequency
            .iter()
            .map(|row| (row.encountered_count, row.last_seen_unix_ms))
            .collect::<Vec<_>>(),
        [(4, Some(500)), (3, Some(800)), (1, Some(900)), (0, None)]
    );
    assert_eq!(
        surfaces(&rows(&storage, &query("", VocabularySort::LastSeen))),
        ["犬", "食べた", "猫", "未読"]
    );
    assert_eq!(
        surfaces(&rows(&storage, &VocabularyQuery::default())),
        ["未読", "犬", "食べた", "猫"]
    );
}

#[test]
fn read_only_reopen_document_deletion_and_erased_vocabulary_do_not_rescan_or_change_history() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫 犬");
    let surviving = import(&mut storage, "猫");
    complete(&mut storage, &imported, 500, true);
    let expected = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    storage
        .delete_document(imported.document.document_id)
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for sort in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
    ] {
        let result = storage.vocabulary_page(&query("", sort), 0).unwrap();
        assert_eq!(result.matching_count, 2);
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| row.encountered_count)
                .sum::<u64>(),
            3
        );
    }
    assert_eq!(
        storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap(),
        expected
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    let cat = expected
        .rows
        .iter()
        .find(|row| row.surface == "猫")
        .unwrap();
    // Schema-level deletion fixture; no unconfirmed product deletion API added.
    storage
        .connection
        .execute("DELETE FROM vocabulary WHERE id = ?", [i64::from(cat.id)])
        .unwrap();
    let after = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    assert_eq!(surfaces(&after.rows), ["犬"]);
    assert!(storage
        .load_paste(surviving.document.document_id)
        .unwrap()
        .is_some());
    assert!(rows(&storage, &query("猫", VocabularySort::Frequency)).is_empty());
    let fresh = register(&mut storage, "猫", 600);
    assert!(fresh > cat.id);
    let fresh = rows(&storage, &query("猫", VocabularySort::Newest))
        .pop()
        .unwrap();
    assert_eq!(fresh.status, VocabularyStatus::New);
    assert_eq!(fresh.encountered_count, 0);
    assert_eq!(fresh.last_seen_unix_ms, None);
}

#[test]
fn pending_legacy_unused_and_failed_completions_and_uncommitted_writes_are_invisible() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫");
    let intent = intent(&mut storage, &imported, 500);
    let pending = storage
        .load_pending_reading_time(intent.document_id, intent.departing_position)
        .unwrap();
    storage
        .save_pending_reading_time(
            pending,
            yomoku_core::active_time::PendingReadingTime::from_millis(123).unwrap(),
        )
        .unwrap();
    let frozen = storage.prepare_page_completion(intent).unwrap();
    storage.record_page_completion(frozen.completion).unwrap();
    storage
        .save_document_reading_position(intent.document_id, intent.departing_position)
        .unwrap();
    let unused = storage.prepare_page_completion(intent).unwrap();
    let expected = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    assert_eq!(expected.rows[0].encountered_count, 0);
    assert!(expected.rows[0].last_seen_unix_ms.is_none());
    storage.connection.execute_batch("CREATE TRIGGER fail_occurrence BEFORE INSERT ON reading_occurrences BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    assert!(storage
        .record_analyzed_page_completion(unused.completion, unused.analysis_id)
        .is_err());
    assert_eq!(
        storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap(),
        expected
    );
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_occurrence")
        .unwrap();
    let observer = Storage::open(database.path()).unwrap();
    storage.connection.execute_batch("BEGIN IMMEDIATE; UPDATE vocabulary SET status = 'ignored'; INSERT INTO vocabulary (surface, status, registered_at_unix_ms) VALUES ('未確定', 'new', 999);").unwrap();
    assert_eq!(
        observer
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap(),
        expected
    );
    storage.connection.execute_batch("ROLLBACK").unwrap();
    storage
        .record_analyzed_page_completion(unused.completion, unused.analysis_id)
        .unwrap();
    let visible = observer
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    assert_eq!(visible.rows[0].encountered_count, 2);
    assert_eq!(visible.rows[0].last_seen_unix_ms, Some(500));
}

#[test]
fn prefix_query_plans_search_existing_surface_and_occurrence_indexes_for_all_sorts() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    for i in 0..300 {
        register(&mut storage, &format!("語{i:03}"), 1);
    }
    for prefix in ["語00", "%", "語\0", "\u{10ffff}"] {
        for sort in [
            VocabularySort::Newest,
            VocabularySort::Frequency,
            VocabularySort::LastSeen,
            VocabularySort::Alphabetical,
        ] {
            let query = query(prefix, sort);
            let (predicate, mut values) = crate::vocabulary_browser::matching(&query);
            let count_sql = format!("SELECT COUNT(*) FROM vocabulary v WHERE {predicate}");
            let page_sql = crate::vocabulary_browser::page_sql(sort, &predicate);
            for (sql, is_page) in [(count_sql, false), (page_sql, true)] {
                if is_page {
                    values.push(Value::Integer(0));
                }
                let plan = storage
                    .connection
                    .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                    .unwrap()
                    .query_map(params_from_iter(&values), |row| row.get::<_, String>(3))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                assert!(
                    plan.iter()
                        .any(|line| line.contains("SEARCH v USING") && line.contains("surface>")),
                    "{sort:?} {prefix:?}: {plan:?}"
                );
                assert!(
                    !plan.iter().any(|line| line.starts_with("SCAN v")),
                    "{plan:?}"
                );
                if is_page {
                    assert!(
                        plan.iter()
                            .filter(|line| line.contains("reading_occurrences_by_entry")
                                && line.contains("vocabulary_id=?"))
                            .count()
                            >= 3,
                        "{plan:?}"
                    );
                    assert!(
                        !plan.iter().any(|line| line.starts_with("SCAN o")),
                        "{plan:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn newest_page_work_does_not_grow_with_offscreen_encounters() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫");
    for index in 0..205 {
        register(&mut storage, &format!("新{index:03}"), 600);
    }
    let query = VocabularyQuery::default();
    let expected = storage.vocabulary_page(&query, 0).unwrap();
    let steps = |storage: &Storage| {
        let (predicate, mut values) = crate::vocabulary_browser::matching(&query);
        values.push(Value::Integer(0));
        let mut statement = storage
            .connection
            .prepare(&crate::vocabulary_browser::page_sql(query.sort, &predicate))
            .unwrap();
        let count = statement
            .query_map(params_from_iter(&values), |_| Ok(()))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len();
        assert_eq!(count, 100);
        statement.get_status(rusqlite::StatementStatus::VmStep)
    };
    let before = steps(&storage);
    for index in 0..100 {
        complete(&mut storage, &imported, 1_000 + index, index % 2 == 0);
    }
    assert_eq!(storage.vocabulary_page(&query, 0).unwrap(), expected);
    // A deterministic work counter, independent of host speed or timing noise.
    // None of these 200 offscreen occurrences belongs to the displayed page.
    assert_eq!(steps(&storage), before);
    let last = storage.vocabulary_page(&query, 200).unwrap();
    assert_eq!(last.matching_count, 206);
    let cat = last.rows.iter().find(|row| row.surface == "猫").unwrap();
    assert_eq!(cat.encountered_count, 200);
    assert_eq!(cat.last_seen_unix_ms, Some(1_099));
    // History validation still applies when the affected row is on the page.
    storage
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    storage
        .connection
        .execute("UPDATE completion_analyses SET semantics_version = 2", [])
        .unwrap();
    assert_eq!(storage.vocabulary_page(&query, 0).unwrap(), expected);
    assert!(storage.vocabulary_page(&query, 200).is_err());
}

#[test]
fn metric_pages_keep_exact_histories_ties_filters_and_unread_rows_across_boundaries() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut expected = Vec::new();
    for index in 0..257 {
        let surface = format!("語{index:03}");
        let date = index % 3;
        expected.push(VocabularyRow {
            id: register(&mut storage, &surface, date),
            surface,
            status: VocabularyStatus::New,
            registered_at_unix_ms: date,
            encountered_count: 0,
            last_seen_unix_ms: None,
        });
    }
    for (group, (count, date)) in [(1, 900), (1, 900), (2, 100), (2, 100), (3, 500)]
        .into_iter()
        .enumerate()
    {
        let text = expected
            .iter()
            .skip(group)
            .step_by(7)
            .map(|row| row.surface.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let imported = import(&mut storage, &text);
        for reread in 0..count {
            // Regressing dates must still sort by the maximum completion date.
            complete(&mut storage, &imported, date - reread, reread % 2 == 0);
        }
        storage
            .delete_document(imported.document.document_id)
            .unwrap();
        for row in expected.iter_mut().skip(group).step_by(7) {
            row.encountered_count = count as u64;
            row.last_seen_unix_ms = Some(date);
        }
    }
    for (index, row) in expected.iter_mut().enumerate() {
        row.status = VocabularyQuery::default().statuses[index % 5];
        storage
            .set_vocabulary_status(row.id, row.status, 1_000)
            .unwrap();
    }
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    for sort in [VocabularySort::Frequency, VocabularySort::LastSeen] {
        for prefix in ["", "語0", "語1", "不存在"] {
            for statuses in [
                VocabularyQuery::default().statuses,
                vec![VocabularyStatus::New, VocabularyStatus::Known],
                vec![],
            ] {
                let query = VocabularyQuery {
                    prefix: prefix.into(),
                    statuses,
                    sort,
                };
                let mut matching: Vec<_> = expected
                    .iter()
                    .filter(|row| {
                        row.surface.starts_with(prefix) && query.statuses.contains(&row.status)
                    })
                    .cloned()
                    .collect();
                matching.sort_by(|a, b| {
                    let metric = match sort {
                        VocabularySort::Frequency => b.encountered_count.cmp(&a.encountered_count),
                        _ => b.last_seen_unix_ms.cmp(&a.last_seen_unix_ms),
                    };
                    metric
                        .then_with(|| b.registered_at_unix_ms.cmp(&a.registered_at_unix_ms))
                        .then_with(|| b.id.cmp(&a.id))
                });
                for offset in [0, 100, 200, 300] {
                    let page = storage.vocabulary_page(&query, offset).unwrap();
                    let slice: Vec<_> = matching
                        .iter()
                        .skip(offset as usize)
                        .take(100)
                        .cloned()
                        .collect();
                    assert_eq!(page.rows, slice, "{query:?} offset {offset}");
                    assert_eq!(page.matching_count, matching.len() as u64);
                    let next = offset + slice.len() as u64;
                    assert_eq!(
                        page.next_offset,
                        (next < matching.len() as u64).then_some(next)
                    );
                    assert!(storage.connection.is_autocommit());
                }
            }
        }
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn metric_page_offscreen_work_is_bounded_to_the_sorting_metric() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫 猫");
    complete(&mut storage, &imported, 1_000, false);
    for index in 0..205 {
        register(&mut storage, &format!("未読{index:03}"), 600);
    }
    let steps = |storage: &Storage, sql: &str, values: &[Value]| {
        let mut statement = storage.connection.prepare(sql).unwrap();
        let mut rows = statement.query(params_from_iter(values)).unwrap();
        while rows.next().unwrap().is_some() {}
        drop(rows);
        statement.get_status(rusqlite::StatementStatus::VmStep)
    };
    let metric_sql = [
        "SELECT COUNT(*) FROM reading_occurrences WHERE vocabulary_id = ?",
        "SELECT MAX(c.completed_at_unix_ms) FROM reading_occurrences o JOIN reading_completions c USING (command_id) WHERE o.vocabulary_id = ?",
    ];
    let cat = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap()
        .id;
    let mut measurements = Vec::new();
    for (sort, metric) in [VocabularySort::Frequency, VocabularySort::LastSeen]
        .into_iter()
        .zip(metric_sql)
    {
        let query = query("", sort);
        let (predicate, mut values) = crate::vocabulary_browser::matching(&query);
        values.push(Value::Integer(200));
        let sql = crate::vocabulary_browser::page_sql(sort, &predicate);
        let page = storage.vocabulary_page(&query, 200).unwrap();
        assert!(page.rows.iter().all(|row| row.encountered_count == 0));
        measurements.push((
            query,
            steps(&storage, &sql, &values),
            steps(&storage, metric, &[Value::Integer(cat.into())]),
            sql,
            values,
            metric,
            page,
        ));
    }
    for index in 0..100 {
        complete(&mut storage, &imported, 1_001 + index, index % 2 == 0);
    }
    for (query, page_before, metric_before, sql, values, metric, expected) in measurements {
        assert_eq!(storage.vocabulary_page(&query, 200).unwrap(), expected);
        let growth = steps(&storage, &sql, &values) - page_before;
        let metric_growth = steps(&storage, metric, &[Value::Integer(cat.into())]) - metric_before;
        // Each of the 200 added offscreen occurrences must pay only for the
        // necessary sorting aggregate, with at most one extra VM step per row.
        // This deterministic work bound rejects probing other summaries or
        // completion/analysis/effective provenance for discarded candidates.
        assert!(
            growth > 0 && growth <= metric_growth + 200,
            "{:?}: page growth {growth}, isolated metric growth {metric_growth}",
            query.sort
        );
        storage
            .connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        storage
            .connection
            .execute("UPDATE completion_analyses SET semantics_version = 2", [])
            .unwrap();
        assert_eq!(storage.vocabulary_page(&query, 200).unwrap(), expected);
        assert!(matches!(
            storage.vocabulary_page(&query, 0),
            Err(StorageError::InvalidCompletion)
        ));
        assert!(storage.connection.is_autocommit());
        storage
            .connection
            .execute("UPDATE completion_analyses SET semantics_version = 1", [])
            .unwrap();
    }
}

#[test]
fn corrupt_values_or_query_failures_release_snapshot_and_never_repair_data() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = import(&mut storage, "猫");
    complete(&mut storage, &imported, 500, true);
    let original = storage
        .vocabulary_page(&VocabularyQuery::default(), 0)
        .unwrap();
    storage
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    for sql in [
        "UPDATE vocabulary SET registered_at_unix_ms = -1",
        "UPDATE vocabulary SET registered_at_unix_ms = 100, status = 'invalid'",
        "UPDATE vocabulary SET status = 'known'; UPDATE completion_analyses SET semantics_version = 2",
        "UPDATE completion_analyses SET semantics_version = 1; UPDATE reading_completions SET completed_at_unix_ms = -1",
        "UPDATE reading_completions SET completed_at_unix_ms = 500; PRAGMA foreign_keys = OFF; UPDATE completion_analyses SET command_id = command_id + 1000",
    ] {
        storage.connection.execute_batch(sql).unwrap();
        let before = fs::read(database.path()).unwrap();
        for sort in [VocabularySort::Newest, VocabularySort::Frequency, VocabularySort::LastSeen, VocabularySort::Alphabetical] {
            assert!(storage.vocabulary_page(&query("", sort), 0).is_err());
            assert!(storage.connection.is_autocommit());
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
    }
    storage.connection.execute_batch("UPDATE completion_analyses SET command_id = command_id - 1000; PRAGMA foreign_keys = ON").unwrap();
    assert_eq!(
        storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap(),
        original
    );
    storage
        .connection
        .execute_batch("ALTER TABLE reading_occurrences RENAME TO unavailable_occurrences")
        .unwrap();
    for sort in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
    ] {
        assert!(matches!(
            storage.vocabulary_page(&query("", sort), 0),
            Err(StorageError::Sqlite(_))
        ));
        assert!(storage.connection.is_autocommit());
    }
    storage
        .connection
        .execute_batch("ALTER TABLE unavailable_occurrences RENAME TO reading_occurrences")
        .unwrap();
    assert_eq!(
        storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap(),
        original
    );
}
