use super::*;
use yomoku_core::{
    completion::PageCompletionIntent,
    reader::ReaderPage,
    statistics::{StatisticsBuckets, StatisticsTimeframe as Timeframe},
};

const NOW: i64 = 1_800_000_000_123;
const DAY: i64 = 86_400_000;

fn seed(database: &Database) -> (Storage, yomoku_core::document::DocumentId) {
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = storage
        .import_analyzed_paste(
            &PasteText::new("猫 猫 犬 鳥 魚 𠮷".into()).unwrap(),
            NOW - 400 * DAY,
            |chapter| {
                let mut analysis = fixture(chapter);
                for token in &mut analysis.tokens {
                    token.dictionary_form = Some(Surface::new(&token.surface).unwrap());
                }
                Ok::<_, DocumentError>(analysis)
            },
        )
        .map(|outcome| match outcome {
            yomoku_storage::PasteImportOutcome::Created(value) => value,
            _ => panic!("expected fresh import"),
        })
        .unwrap();
    let id = imported.document.document_id;
    let snapshot = storage.load_document_reader_snapshot(id).unwrap().unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();

    let intent = PageCompletionIntent {
        document_id: id,
        page: page.anchor,
        page_length: snapshot.page_length,
        departing_position: page.position,
        completed_at_unix_ms: NOW - 60 * DAY,
        analysis_id: imported.analysis_id,
    };
    let command = storage_fixture::prepare(&mut storage, intent).unwrap();
    storage.record_effective_page_completion(&command).unwrap();
    for (surface, status) in [
        ("猫", VocabularyStatus::New),
        ("犬", VocabularyStatus::Learning),
        ("鳥", VocabularyStatus::Familiar),
        ("魚", VocabularyStatus::Known),
        ("𠮷", VocabularyStatus::Ignored),
    ] {
        let entry = storage
            .vocabulary_entry(&Surface::new(surface).unwrap())
            .unwrap()
            .unwrap();
        storage
            .set_vocabulary_status(entry.id, status, NOW - DAY)
            .unwrap();
    }
    (storage, id)
}

fn read(router: &EffectRouter, frame: Timeframe) -> StatisticsBuckets {
    let (sender, receiver) = mpsc::channel();
    router.update(
        Event::SetStatisticsTimeframe {
            timeframe: frame,
            as_of_unix_ms: NOW,
        },
        move |_| {
            let _ = sender.send(());
        },
    );
    while router.view().storage_busy {
        receiver.recv_timeout(TIMEOUT).unwrap();
    }
    let view = router.view();
    assert_eq!(view.statistics.error, None);
    assert!(!view.statistics.loading);
    view.statistics.data.unwrap()
}

#[test]
fn statistics_fifo_returns_exact_all_timeframes_and_persists_choice_without_loading_analyzer() {
    let database = Database::new();
    let (mut observer, id) = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    for frame in [
        Timeframe::LastWeek,
        Timeframe::LastYear,
        Timeframe::LastMonth,
        Timeframe::LastSixMonths,
        Timeframe::AllTime,
    ] {
        let result = read(&router, frame);
        assert_eq!(result, observer.statistics_buckets(frame, NOW).unwrap());
        let last = result.buckets.last().unwrap().values;
        assert_eq!(last.reading.japanese_occurrences, 6);

        assert_eq!(
            last.statuses,
            yomoku_core::library::WordStatusCounts {
                new: 1,
                learning: 1,
                familiar: 1,
                known: 1,
                ignored: 1,
            }
        );

        if frame == Timeframe::LastMonth {
            assert_eq!(result.opening.reading.japanese_occurrences, 6);
        }
        assert_eq!(observer.statistics_timeframe().unwrap(), frame);
        assert!(trace.try_recv().is_err());
    }
    let retained = router.view().statistics.data.unwrap();
    // Document deletion retains history. A newly created router/reopened worker
    // projects the exact same values without loading content or dictionaries.
    observer.delete_document(id).unwrap();
    let Factory {
        loader,
        trace: reopened_trace,
        ..
    } = Factory::new(false, None);
    let (reopened, reopened_lifecycle) = import_router(database.path(), loader).unwrap();
    dispatch(&reopened, Event::LoadStatisticsTimeframe);
    assert_eq!(reopened.view().statistics.timeframe, Timeframe::AllTime);
    reopened.update(Event::SelectDestination(Destination::Statistics), |_| {});
    assert_eq!(read(&reopened, Timeframe::AllTime), retained);
    let cat = observer
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    observer
        .set_vocabulary_status(cat.id, VocabularyStatus::Known, NOW)
        .unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    assert!(router.view().statistics.data.is_none());
    let refreshed = read(&router, Timeframe::AllTime);
    assert_eq!(refreshed.buckets.last().unwrap().values.statuses.known, 2);
    assert!(trace.try_recv().is_err() && reopened_trace.try_recv().is_err());
    drop((router, lifecycle, reopened, reopened_lifecycle));
}

#[test]
fn statistics_worker_corruption_and_bad_dates_fail_read_only_then_retry_exact_query() {
    let database = Database::new();
    let (_observer, _) = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    // Open/validate before simulating later corruption; the open-time integrity
    // audit would otherwise reject the entire database before this projection.
    dispatch(&router, Event::RefreshLibrary);
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("PRAGMA ignore_check_constraints = ON; UPDATE reading_completions SET semantics_version = 2; PRAGMA ignore_check_constraints = OFF;").unwrap();
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    let before = std::fs::read(database.path()).unwrap();
    let failed = dispatch(&router, Event::LoadStatistics { as_of_unix_ms: NOW });
    assert_eq!(
        failed.statistics.error,
        Some(DocumentError::IncompatibleDatabase)
    );
    assert!(failed.statistics.data.is_none() && failed.document_error.is_none());
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
    connection
        .execute_batch("UPDATE reading_completions SET semantics_version = 1")
        .unwrap();
    let retried = dispatch(&router, Event::RetryStatistics);
    assert_eq!(retried.statistics.query.unwrap().as_of_unix_ms, NOW);
    assert_eq!(
        retried
            .statistics
            .data
            .unwrap()
            .buckets
            .last()
            .unwrap()
            .values
            .reading
            .japanese_occurrences,
        6
    );
    let failed = dispatch(&router, Event::LoadStatistics { as_of_unix_ms: -1 });
    assert_eq!(
        failed.statistics.error,
        Some(DocumentError::IncompatibleDatabase)
    );
    assert!(failed.statistics.data.is_none());
    assert!(trace.try_recv().is_err());
    drop((router, lifecycle));
}

#[test]
fn statistics_refreshes_after_reader_status_and_reread_completion() {
    use yomoku_core::reader::ReaderTokenKey;
    let database = Database::new();
    let (_observer, id) = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    assert_eq!(
        dispatch(&router, Event::LoadVocabulary)
            .vocabulary
            .matching_count,
        Some(5)
    );
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    read(&router, Timeframe::AllTime);
    router.update(Event::SelectDestination(Destination::Library), |_| {});
    let opened = dispatch(&router, Event::OpenReader(id));
    assert!(opened.statistics.data.is_some());
    let page = opened.reader.unwrap();
    let key = ReaderTokenKey {
        document_id: id,
        page: page.anchor,
        analysis: opened.reader_analysis.unwrap(),
    };
    let tokens = dispatch(&router, Event::LoadReaderTokens(key))
        .effective_reader_tokens
        .unwrap();
    let cat = tokens
        .tokens
        .iter()
        .find(|token| token.surface == "猫")
        .unwrap()
        .vocabulary
        .unwrap();
    let changed = dispatch(
        &router,
        Event::SetEffectiveReaderStatus {
            selection: yomoku_core::effective_session::EffectiveReaderSelection {
                page: tokens.identity,
                segment_index: tokens
                    .tokens
                    .iter()
                    .find(|t| t.surface == "猫")
                    .unwrap()
                    .segment_index,
            },
            vocabulary_id: cat.id,
            status: VocabularyStatus::Known,
            occurred_at_unix_ms: NOW,
        },
    );
    assert!(changed.statistics.data.is_none());
    assert_eq!(changed.vocabulary.matching_count, None);
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    assert_eq!(
        read(&router, Timeframe::AllTime)
            .buckets
            .last()
            .unwrap()
            .values
            .statuses
            .known,
        2
    );
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    dispatch(&router, Event::LoadVocabulary);
    router.update(Event::SelectDestination(Destination::Library), |_| {});
    let page = dispatch(&router, Event::OpenReader(id)).reader.unwrap();
    let tokens = dispatch(
        &router,
        Event::LoadReaderTokens(ReaderTokenKey {
            document_id: id,
            page: page.anchor,
            analysis: router.view().reader_analysis.unwrap(),
        }),
    )
    .effective_reader_tokens
    .unwrap();
    let (sender, receiver) = mpsc::channel();
    router.update(
        Event::CompleteEffectiveReaderPage {
            identity: tokens.identity,
            position: page.position,
            completed_at_unix_ms: NOW,
        },
        move |_| {
            let _ = sender.send(());
        },
    );
    loop {
        receiver.recv_timeout(TIMEOUT).unwrap();
        let view = router.view();
        if !view.storage_busy {
            assert!(view.statistics.data.is_none() && view.statistics.query.is_none());
            assert_eq!(view.vocabulary.matching_count, None);
            assert_eq!(view.document_error, None);
            break;
        }
    }
    router.update(Event::SelectDestination(Destination::Statistics), |_| {});
    let refreshed = read(&router, Timeframe::AllTime);
    assert_eq!(
        refreshed
            .buckets
            .last()
            .unwrap()
            .values
            .reading
            .japanese_occurrences,
        12
    );

    assert!(trace.try_recv().is_err());
    drop((router, lifecycle));
}
