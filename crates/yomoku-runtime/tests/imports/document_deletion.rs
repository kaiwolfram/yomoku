use super::*;

#[test]
fn confirmed_document_deletion_runs_on_fifo_without_analysis_and_retries_transaction_failure() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let created = dispatch(&router, Event::CreatePaste("猫 犬 𠮷".into()));
    let id = created.import_summary.unwrap().document_id;
    trace.recv_timeout(TIMEOUT).unwrap(); // load
    trace.recv_timeout(TIMEOUT).unwrap(); // analyze
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER fail BEFORE DELETE ON documents BEGIN SELECT RAISE(ABORT, 'delete'); END;").unwrap();
    router.update(Event::RequestDocumentDeletion(id), |_| {});
    assert_eq!(router.view().library_delete_target.unwrap().document_id, id);
    let failed = dispatch(&router, Event::ConfirmDocumentDeletion(id));
    assert_eq!(
        failed.library_delete_error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(failed.library_document_count, 1);
    assert!(Storage::open(database.path())
        .unwrap()
        .load_document_reader_snapshot(id)
        .unwrap()
        .is_some());
    connection.execute_batch("DROP TRIGGER fail").unwrap();
    let deleted = dispatch(&router, Event::ConfirmDocumentDeletion(id));
    assert!(deleted.library_delete_target.is_none());
    assert_eq!(deleted.library_document_count, 0);
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(storage.load_document_reader_snapshot(id).unwrap().is_none());
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    assert!(trace.try_recv().is_err(), "deletion must not load/analyze");
    let created = dispatch(&router, Event::CreatePaste("猫 犬 𠮷".into()));
    let new_id = created.import_summary.unwrap().document_id;
    assert!(new_id > id);
    assert_eq!(created.documents[0].progress_percent, 0);
    assert!(!created.documents[0].finished_at_least_once);
    assert_eq!(created.documents[0].last_read_sequence, None);
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn confirmed_bulk_deletion_uses_one_fifo_transaction_and_exact_selection_on_retry() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    let first = dispatch(&router, Event::CreatePaste("猫 犬".into()))
        .import_summary
        .unwrap()
        .document_id;
    let last = dispatch(&router, Event::CreatePaste("犬 猫".into()))
        .import_summary
        .unwrap()
        .document_id;
    let survivor = dispatch(&router, Event::CreatePaste("猫 𠮷".into()))
        .import_summary
        .unwrap()
        .document_id;
    for _ in 0..4 {
        trace.recv_timeout(TIMEOUT).unwrap();
    } // load + three analyses
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch(&format!("CREATE TRIGGER fail BEFORE DELETE ON documents WHEN OLD.id = {} BEGIN SELECT RAISE(ABORT, 'last document'); END;", i64::from(last))).unwrap();
    for id in [first, last] {
        router.update(Event::ToggleLibraryDocumentSelection(id), |_| {});
    }
    router.update(Event::RequestSelectedDocumentDeletion, |_| {});
    let confirm = Event::ConfirmSelectedDocumentDeletion(vec![first, last]);
    let failed = dispatch(&router, confirm.clone());
    assert_eq!(
        failed.library_delete_error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(failed.library_selection, Some(vec![first, last]));
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .list_documents()
            .unwrap()
            .len(),
        3
    );
    router.update(Event::SearchLibraryTitles("𠮷".into()), |_| {});
    connection.execute_batch("DROP TRIGGER fail").unwrap();
    let deleted = dispatch(&router, confirm);
    assert!(deleted.library_selection.is_none());
    assert!(deleted.library_bulk_delete_targets.is_none());
    assert_eq!(deleted.documents[0].document_id, survivor);
    assert_eq!(deleted.library_document_count, 1);
    assert_eq!(deleted.library_title_query, "𠮷");
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.list_documents().unwrap().len(), 1);
    assert_eq!(
        storage
            .vocabulary_entry(&Surface::new("犬").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    assert!(
        trace.try_recv().is_err(),
        "bulk deletion must not load/analyze"
    );
    finish_worker(router, lifecycle, &trace);
}
