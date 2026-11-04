use super::*;
use yomoku_core::{document::DocumentId, reader_images::ReaderImageAsset};

fn asset(path: &str) -> ReaderImageAsset {
    ReaderImageAsset {
        path: path.into(),
        media_type: "image/png".into(),
    }
}

#[test]
fn asset_fifo_reads_reopen_without_dictionaries_renders_or_state_changes() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let yomoku_storage::EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>English</p>"),
            "assets.epub",
            1,
            |chapter| Ok::<_, std::convert::Infallible>(fixture(chapter)),
        )
        .unwrap()
    else {
        panic!()
    };
    let id = imported.document.document_id;
    let before = totals(&database.path());
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = yomoku_runtime::effect_router(
        database.path(),
        loader,
        Box::new(|| panic!("asset reads must not load word index")),
    )
    .unwrap();
    let view = router.view();
    let (send, receive) = mpsc::channel();
    let mut expected = Vec::new();
    for (path, image) in imported.document.images {
        expected.push(Ok(image));
        let send = send.clone();
        lifecycle
            .load_epub_image_asset(id, asset(&path), move |result| {
                send.send((thread::current().id(), result)).unwrap();
            })
            .unwrap();
    }
    for key in [asset("OPS/missing.png"), asset("OPS/../bad.png")] {
        expected.push(Err(if key.path.contains("..") {
            DocumentError::InvalidDocument
        } else {
            DocumentError::NotFound
        }));
        let send = send.clone();
        lifecycle
            .load_epub_image_asset(id, key, move |result| {
                send.send((thread::current().id(), result)).unwrap();
            })
            .unwrap();
    }
    let mut worker = None;
    for expected in expected {
        let (current, result) = receive.recv_timeout(TIMEOUT).unwrap();
        assert_ne!(current, thread::current().id());
        assert_eq!(*worker.get_or_insert(current), current);
        assert_eq!(result, expected);
    }
    assert_eq!(router.view(), view);
    assert_eq!(totals(&database.path()), before);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn queued_asset_reads_follow_import_and_drain_after_router_close() {
    for close in [false, true] {
        let database = Database::new();
        let (release, gate) = mpsc::channel();
        let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
        let (router, lifecycle) = import_router(database.path(), loader).unwrap();
        let (send, receive) = mpsc::channel();
        let imported = send.clone();
        router.update(
            epub::event(&epub_fixture::book(true, "<p>English</p>")),
            move |_| {
                imported.send("import").unwrap();
            },
        );
        let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
            Trace::Load(worker) => worker,
            other => panic!("{other:?}"),
        };
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
        let (result_send, result_receive) = mpsc::channel();
        lifecycle
            .load_epub_image_asset(
                DocumentId::try_from(1).unwrap(),
                asset("OPS/images/挿絵.png"),
                move |result| {
                    assert_eq!(thread::current().id(), worker);
                    result_send.send(result).unwrap();
                    send.send("image").unwrap();
                },
            )
            .unwrap();
        assert!(receive.try_recv().is_err());
        let retained = if close {
            drop(router);
            None
        } else {
            Some(router)
        };
        if close {
            assert!(lifecycle
                .load_epub_image_asset(
                    DocumentId::try_from(1).unwrap(),
                    asset("OPS/images/cover.png"),
                    |_| panic!("closed")
                )
                .is_err());
        }
        release.send(()).unwrap();
        if !close {
            assert_eq!(receive.recv_timeout(TIMEOUT).unwrap(), "import");
        }
        assert_eq!(receive.recv_timeout(TIMEOUT).unwrap(), "image");
        assert_eq!(
            result_receive.recv_timeout(TIMEOUT).unwrap().unwrap().bytes,
            b"fixture image bytes"
        );
        drop(retained);
        drain_trace(&trace);
    }
}

#[test]
fn asset_open_failure_is_retryable_and_invalid_keys_do_not_create_database() {
    let directory = Database::new();
    let path = directory.0.join("missing").join("assets.sqlite3");
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = import_router(path.clone(), loader).unwrap();
    let id = DocumentId::try_from(1).unwrap();
    for (key, expected) in [
        (asset("../bad"), DocumentError::InvalidDocument),
        (asset("OPS/image.png"), DocumentError::Unavailable),
    ] {
        let (send, receive) = mpsc::channel();
        lifecycle
            .load_epub_image_asset(id, key, move |result| send.send(result).unwrap())
            .unwrap();
        assert_eq!(receive.recv_timeout(TIMEOUT).unwrap(), Err(expected));
        assert!(!path.exists());
    }
    std::fs::create_dir(path.parent().unwrap()).unwrap();
    let (send, receive) = mpsc::channel();
    lifecycle
        .load_epub_image_asset(id, asset("OPS/image.png"), move |result| {
            send.send(result).unwrap()
        })
        .unwrap();
    assert_eq!(
        receive.recv_timeout(TIMEOUT).unwrap(),
        Err(DocumentError::NotFound)
    );
    assert!(path.exists());
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn cover_fifo_reads_metadata_without_dictionaries_renders_or_state_changes() {
    let database = Database::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let yomoku_storage::EpubImportOutcome::Created(imported) = storage
        .import_analyzed_epub(
            &epub_fixture::book(true, "<p>English</p>"),
            "cover.epub",
            1,
            |chapter| Ok::<_, std::convert::Infallible>(fixture(chapter)),
        )
        .unwrap()
    else {
        panic!()
    };
    let id = imported.document.document_id;
    let expected = storage.load_epub_cover_reference(id).unwrap();
    let before = totals(&database.path());
    drop(storage);
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = yomoku_runtime::effect_router(
        database.path(),
        loader,
        Box::new(|| panic!("cover read loaded index")),
    )
    .unwrap();
    let view = router.view();
    let (send, receive) = mpsc::channel();
    for document in [id, DocumentId::try_from(i64::MAX).unwrap(), id] {
        let send = send.clone();
        lifecycle
            .load_epub_cover_reference(document, move |result| {
                send.send((thread::current().id(), result)).unwrap();
            })
            .unwrap();
    }
    let mut worker = None;
    for expected in [expected.clone(), None, expected] {
        let (current, result) = receive.recv_timeout(TIMEOUT).unwrap();
        assert_ne!(current, thread::current().id());
        assert_eq!(*worker.get_or_insert(current), current);
        assert_eq!(result, Ok(expected));
    }
    assert_eq!(router.view(), view);
    assert_eq!(totals(&database.path()), before);
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}

#[test]
fn cover_fifo_waits_for_import_and_drains_after_close() {
    let database = Database::new();
    let (release, gate) = mpsc::channel();
    let Factory { loader, trace, .. } = Factory::new(false, Some(gate));
    let (router, lifecycle) = import_router(database.path(), loader).unwrap();
    router.update(
        epub::event(&epub_fixture::book(true, "<p>English</p>")),
        |_| {},
    );
    let worker = match trace.recv_timeout(TIMEOUT).unwrap() {
        Trace::Load(worker) => worker,
        other => panic!("{other:?}"),
    };
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), Trace::Analyze(worker));
    let id = DocumentId::try_from(1).unwrap();
    let (send, receive) = mpsc::channel();
    lifecycle
        .load_epub_cover_reference(id, move |result| {
            assert_eq!(thread::current().id(), worker);
            send.send(result).unwrap();
        })
        .unwrap();
    assert!(receive.try_recv().is_err());
    drop(router);
    assert!(lifecycle
        .load_epub_cover_reference(id, |_| panic!("closed"))
        .is_err());
    release.send(()).unwrap();
    let cover = receive.recv_timeout(TIMEOUT).unwrap().unwrap().unwrap();
    assert_eq!(cover.asset, asset("OPS/images/cover.png"));
    assert_eq!(cover.sha256.len(), 32);
    drain_trace(&trace);
}

#[test]
fn cover_open_failure_can_retry_without_loading_dictionaries() {
    let directory = Database::new();
    let path = directory.0.join("missing").join("covers.sqlite3");
    let Factory { loader, trace, .. } = Factory::new(true, None);
    let (router, lifecycle) = import_router(path.clone(), loader).unwrap();
    let id = DocumentId::try_from(1).unwrap();
    let (send, receive) = mpsc::channel();
    let first = send.clone();
    lifecycle
        .load_epub_cover_reference(id, move |result| first.send(result).unwrap())
        .unwrap();
    assert_eq!(
        receive.recv_timeout(TIMEOUT).unwrap(),
        Err(DocumentError::Unavailable)
    );
    assert!(!path.exists());
    std::fs::create_dir(path.parent().unwrap()).unwrap();
    lifecycle
        .load_epub_cover_reference(id, move |result| send.send(result).unwrap())
        .unwrap();
    assert_eq!(receive.recv_timeout(TIMEOUT).unwrap(), Ok(None));
    assert!(path.exists());
    assert!(trace.try_recv().is_err());
    finish_worker(router, lifecycle, &trace);
}
