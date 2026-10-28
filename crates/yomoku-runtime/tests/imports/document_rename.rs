use super::*;

#[test]
fn document_rename_fifo_retries_failure_without_analysis_and_reports_deleted_target() {
    let database = Database::new();
    let Factory { loader, trace, .. } = Factory::new(false, None);
    let (router, lifecycle) = effect_router_with_analyzed_imports(database.path(), loader).unwrap();
    let text = "猫 犬 𠮷";
    let created = dispatch(&router, Event::CreatePaste(text.into()));
    let id = created.import_summary.unwrap().document_id;
    trace.recv_timeout(TIMEOUT).unwrap();
    trace.recv_timeout(TIMEOUT).unwrap();
    let connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER fail BEFORE UPDATE OF title ON documents BEGIN SELECT RAISE(ABORT, 'rename'); END;").unwrap();
    router.update(Event::RequestDocumentRename(id), |_| {});
    let title = " 改題𠮷 か\u{3099}\0'\n ";
    router.update(
        Event::EditDocumentTitle {
            document_id: id,
            title: title.into(),
        },
        |_| {},
    );
    let failed = dispatch(&router, Event::SaveDocumentTitle(id));
    assert_eq!(
        failed.library_rename.unwrap().error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(failed.documents[0].title, text);
    connection.execute_batch("DROP TRIGGER fail").unwrap();
    let saved = dispatch(&router, Event::SaveDocumentTitle(id));
    assert!(saved.library_rename.is_none());
    assert_eq!(saved.documents[0].title, title);
    assert_eq!(saved.import_summary.unwrap().title, title);
    let duplicate = dispatch(&router, Event::CreatePaste(text.into()));
    assert_eq!(duplicate.documents[0].title, title);
    assert_eq!(duplicate.import_summary.unwrap().document_id, id);
    assert!(
        trace.try_recv().is_err(),
        "rename/duplicate must not load/analyze"
    );
    router.update(Event::RequestDocumentRename(id), |_| {});
    Storage::open(database.path())
        .unwrap()
        .delete_document(id)
        .unwrap();
    let missing = dispatch(&router, Event::SaveDocumentTitle(id));
    assert_eq!(
        missing.library_rename.unwrap().error,
        Some(DocumentError::NotFound)
    );
    assert!(Storage::open(database.path())
        .unwrap()
        .list_documents()
        .unwrap()
        .is_empty());
    finish_worker(router, lifecycle, &trace);
}
