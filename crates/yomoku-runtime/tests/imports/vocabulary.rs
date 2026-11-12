use super::*;
use yomoku_core::vocabulary::{VocabularyQuery, VocabularySort};

fn seed(database: &Database) -> Storage {
    let mut storage = Storage::open(database.path()).unwrap();
    for index in 0..205 {
        storage
            .register_vocabulary(&[Surface::new(format!("猫{index:03}")).unwrap()], index)
            .unwrap();
    }
    storage
}

#[test]
fn vocabulary_fifo_pages_match_repository_and_never_load_analyzer_or_write() {
    let database = Database::new();
    let observer = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    let before = std::fs::read(database.path()).unwrap();
    for sort in [
        VocabularySort::Newest,
        VocabularySort::Frequency,
        VocabularySort::LastSeen,
        VocabularySort::Alphabetical,
        VocabularySort::Oldest,
        VocabularySort::FrequencyAscending,
        VocabularySort::LastSeenAscending,
        VocabularySort::AlphabeticalDescending,
    ] {
        let query = VocabularyQuery {
            sort,
            ..Default::default()
        };
        let mut view = dispatch(&router, Event::SetVocabularyQuery(query.clone()));
        let mut expected_rows = vec![];
        for offset in [0, 100, 200] {
            if offset > 0 {
                view = dispatch(&router, Event::LoadMoreVocabulary);
            }
            let page = observer.vocabulary_page(&query, offset).unwrap();
            expected_rows.extend(page.rows);
            assert_eq!(view.vocabulary.rows, expected_rows);
            assert_eq!(view.vocabulary.matching_count, Some(205));
            assert_eq!(view.vocabulary.next_offset, page.next_offset);
            assert_eq!(view.vocabulary.error, None);
        }
    }
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
    assert!(trace.try_recv().is_err());
    drop((router, lifecycle));
}

#[test]
fn vocabulary_worker_failed_append_retries_exact_page_without_rescan_then_refreshes_external_changes(
) {
    let database = Database::new();
    let mut observer = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    let query = VocabularyQuery {
        sort: VocabularySort::Alphabetical,
        ..Default::default()
    };
    let first = dispatch(&router, Event::SetVocabularyQuery(query.clone()));
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("PRAGMA ignore_check_constraints=ON; UPDATE vocabulary SET status='bad' WHERE surface='猫150'; PRAGMA ignore_check_constraints=OFF;").unwrap();
    let before = std::fs::read(database.path()).unwrap();
    let failed = dispatch(&router, Event::LoadMoreVocabulary);
    assert!(failed.vocabulary.error.is_some());
    assert!(failed.document_error.is_none());
    assert_eq!(failed.vocabulary.rows, first.vocabulary.rows);
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
    connection
        .execute_batch("UPDATE vocabulary SET status='new' WHERE surface='猫150'")
        .unwrap();
    let retried = dispatch(&router, Event::RetryVocabulary);
    assert_eq!(retried.vocabulary.rows.len(), 200);
    assert_eq!(retried.vocabulary.error, None);
    let cat = observer
        .vocabulary_entry(&Surface::new("猫000").unwrap())
        .unwrap()
        .unwrap();
    observer
        .set_vocabulary_status(cat.id, VocabularyStatus::Known, 999)
        .unwrap();
    router.update(Event::SelectDestination(Destination::Library), |_| {});
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    let refreshed = dispatch(&router, Event::LoadVocabulary);
    assert_eq!(refreshed.vocabulary.query, query);
    assert_eq!(refreshed.vocabulary.rows.len(), 100);
    assert_eq!(refreshed.vocabulary.rows[0].status, VocabularyStatus::Known);
    assert!(trace.try_recv().is_err());
    drop((router, lifecycle));
}

#[test]
fn vocabulary_external_count_changes_restart_pages_and_reopen_reads_literal_filters() {
    let database = Database::new();
    let mut observer = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    dispatch(&router, Event::LoadVocabulary);
    observer
        .register_vocabulary(&[Surface::new("𠮷\0_%e\u{301}").unwrap()], 999)
        .unwrap();
    // A count change causes an additional FIFO read of page zero. Wait for both.
    let (sender, receiver) = mpsc::channel();
    router.update(Event::LoadMoreVocabulary, move |_| {
        let _ = sender.send(());
    });
    loop {
        receiver.recv_timeout(TIMEOUT).unwrap();
        let view = router.view();
        if !view.storage_busy {
            assert_eq!(view.vocabulary.rows.len(), 100);
            assert_eq!(view.vocabulary.matching_count, Some(206));
            assert_eq!(view.vocabulary.rows[0].surface, "𠮷\0_%e\u{301}");
            break;
        }
    }
    drop((router, lifecycle));
    let Factory {
        loader,
        trace: reopened_trace,
        ..
    } = Factory::new(false, None);
    let (reopened, lifecycle) = import_router(database.path(), loader).unwrap();
    reopened.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    for statuses in [
        vec![VocabularyStatus::New],
        vec![VocabularyStatus::Known],
        vec![],
    ] {
        let query = VocabularyQuery {
            prefix: "𠮷\0_%".into(),
            statuses,
            ..Default::default()
        };
        let view = dispatch(&reopened, Event::SetVocabularyQuery(query.clone()));
        let expected = observer.vocabulary_page(&query, 0).unwrap();
        assert_eq!(view.vocabulary.rows, expected.rows);
        assert_eq!(
            view.vocabulary.matching_count,
            Some(expected.matching_count)
        );
    }
    assert!(trace.try_recv().is_err() && reopened_trace.try_recv().is_err());
    drop((reopened, lifecycle));
}

#[test]
fn vocabulary_bulk_fifo_frozen_status_retry_rolls_back_all_rows_and_excludes_later_matches() {
    use yomoku_core::vocabulary::VocabularyBulkAction;
    let database = Database::new();
    let mut observer = seed(&database);
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(Event::SelectDestination(Destination::Vocabulary), |_| {});
    let first = dispatch(&router, Event::LoadVocabulary);
    assert_eq!(first.vocabulary.rows.len(), 100);
    let before = std::fs::read(database.path()).unwrap();
    let captured = dispatch(&router, Event::SelectAllVocabulary);
    let ids = captured.vocabulary.selection.unwrap();
    assert_eq!(ids.len(), 205);
    router.update(
        Event::RequestVocabularyBulkAction(VocabularyBulkAction::SetStatus {
            status: VocabularyStatus::Known,
            occurred_at_unix_ms: 999,
        }),
        |_| {},
    );
    let command = router.view().vocabulary.confirmation.unwrap();
    assert_eq!(command.ids, ids);
    assert_eq!(std::fs::read(database.path()).unwrap(), before);
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER bulk_fail BEFORE UPDATE ON vocabulary WHEN OLD.surface='猫150' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let failed = dispatch(&router, Event::ConfirmVocabularyBulkAction(command.clone()));
    assert_eq!(failed.vocabulary.confirmation, Some(command.clone()));
    assert!(failed.vocabulary.bulk_error.is_some() && failed.document_error.is_none());
    assert_eq!(
        observer
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap()
            .rows,
        first.vocabulary.rows
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM vocabulary_status_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        205
    );
    // Even if matching changes between failure and Retry, the original payload owns the operation.
    connection.execute_batch("DROP TRIGGER bulk_fail").unwrap();
    observer.delete_vocabulary_entries(&[ids[0]]).unwrap();
    observer
        .register_vocabulary(
            &[
                Surface::new("猫000").unwrap(),
                Surface::new("猫later").unwrap(),
            ],
            1000,
        )
        .unwrap();
    let retried = dispatch(&router, Event::RetryVocabularyBulkAction);
    assert!(retried.vocabulary.confirmation.is_none() && retried.vocabulary.bulk_error.is_none());
    assert!(retried.vocabulary.rows.is_empty() && retried.vocabulary.matching_count.is_none());
    assert_eq!(connection.query_row("SELECT COUNT(*) FROM vocabulary_status_events WHERE new_status='known' AND occurred_at_unix_ms=999", [], |row| row.get::<_, i64>(0)).unwrap(), 204);
    assert_eq!(
        observer
            .vocabulary_entry(&Surface::new("猫000").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    assert_eq!(
        observer
            .vocabulary_entry(&Surface::new("猫later").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    let refreshed = dispatch(&router, Event::LoadVocabulary);
    assert_eq!(refreshed.vocabulary.matching_count, Some(206));
    assert_eq!(
        refreshed.vocabulary.rows,
        observer
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap()
            .rows
    );
    assert!(trace.try_recv().is_err());
    drop((router, lifecycle));
}
