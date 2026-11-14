use rusqlite::types::Value;
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::PasteText,
    reader::ReaderPage,
    statistics::StatisticsTimeframe,
    vocabulary::{Surface, VocabularyQuery, VocabularyStatus},
};

use super::*;

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
        .analyzed_fixture(&PasteText::new(text.into()).unwrap(), date, |chapter| {
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
fn later_delete_cascade_and_commit_failures_preserve_history_charts_and_exact_retry() {
    for sql in [
        "CREATE TRIGGER fail BEFORE DELETE ON vocabulary WHEN OLD.surface = '犬' BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON reading_occurrences WHEN OLD.vocabulary_id = 2 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON vocabulary_status_events WHEN OLD.vocabulary_id = 2 BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES vocabulary(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER DELETE ON vocabulary WHEN OLD.surface = '犬' BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = import(&mut storage, "猫 猫 犬", 100);
        let intent = intent(&mut storage, &imported);
        let command = storage.prepare_fixture_completion(intent).unwrap();
        storage.record_effective_page_completion(&command).unwrap();
        let ids = storage.vocabulary_page(&VocabularyQuery::default(), 0).unwrap().rows.into_iter().map(|row| row.id).collect::<Vec<_>>();
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
    {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let live = import(&mut storage, "猫 猫 犬", 100);
        let deleted_document = import(&mut storage, "猫 𠮷", 100);
        let intent = intent(&mut storage, &live);

        let frozen = storage.prepare_fixture_completion(intent).unwrap();
        let complete = |storage: &mut Storage| storage.record_effective_page_completion(&frozen);
        assert_eq!(
            complete(&mut storage).unwrap(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 2 }
        );
        storage
            .delete_document(deleted_document.document.document_id)
            .unwrap();
        let before_totals = storage.lifetime_reading_totals().unwrap();
        assert_eq!(before_totals.japanese_occurrences, 3);

        let cat = storage
            .vocabulary_page(
                &VocabularyQuery {
                    prefix: "猫".into(),
                    ..Default::default()
                },
                0,
            )
            .unwrap()
            .rows
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
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

        let timeline = storage.vocabulary_status_timeline().unwrap();
        assert_eq!(timeline[0].counts.new, 2);
        assert_eq!(timeline.last().unwrap().counts.known, 1);

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
            .set_vocabulary_status(cat[0], VocabularyStatus::Ignored, 900)
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
        let new_command = storage.prepare_fixture_completion(new_intent).unwrap();
        storage
            .record_effective_page_completion(&new_command)
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
