use super::{imports::fixture, *};
use yomoku_core::{completion::PageCompletionIntent, document::PasteText, reader::ReaderPage};
use yomoku_import::txt::TextEncoding;

// Compare every persisted field outside the one explicitly mutable live table.
fn retained_rows(storage: &Storage) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    let mut names = storage.connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name != 'documents' ORDER BY name").unwrap();
    let names: Vec<String> = names
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    names
        .iter()
        .map(|name| {
            let mut statement = storage
                .connection
                .prepare(&format!(
                    "SELECT * FROM \"{}\" ORDER BY 1",
                    name.replace('"', "\"\"")
                ))
                .unwrap();
            let count = statement.column_count();
            statement
                .query_map([], |r| (0..count).map(|i| r.get(i)).collect())
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        })
        .collect()
}

#[test]
fn document_rename_preserves_all_content_provenance_history_time_statuses_and_duplicate_identity() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = PasteText::new("原題\n猫 猫 犬".into()).unwrap();
    let PasteImportOutcome::Created(imported) =
        storage.import_analyzed_paste(&text, 100, fixture).unwrap()
    else {
        panic!("created")
    };
    let id = imported.document.document_id;
    let snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();

    let prepared = storage
        .prepare_fixture_completion(PageCompletionIntent {
            document_id: id,
            page: page.anchor,
            page_length: snapshot.page_length,
            departing_position: page.position,
            completed_at_unix_ms: 500,
            analysis_id: imported.analysis_id,
        })
        .unwrap();
    storage.record_effective_page_completion(&prepared).unwrap();
    storage
        .save_document_reading_position(id, page.position)
        .unwrap();

    let PasteImportOutcome::Created(survivor) = storage
        .import_analyzed_paste(&PasteText::new("別の本".into()).unwrap(), 600, fixture)
        .unwrap()
    else {
        panic!("created")
    };
    let survivor = survivor.document;
    let before = retained_rows(&storage);
    let original = storage.load_document_reader_snapshot(id).unwrap().unwrap();
    let summary = storage
        .list_documents()
        .unwrap()
        .into_iter()
        .find(|d| d.document_id == id)
        .unwrap();
    let title = "  新題𠮷 か\u{3099} '\"; DROP TABLE documents;--\0\n終  ";
    assert!(storage.rename_document(id, title).unwrap());
    assert_eq!(retained_rows(&storage), before);
    let mut expected = original;
    expected.document.title = title.into();
    assert_eq!(
        storage.load_document_reader_snapshot(id).unwrap(),
        Some(expected.clone())
    );
    assert_eq!(
        storage
            .list_documents()
            .unwrap()
            .into_iter()
            .find(|d| d.document_id == id)
            .unwrap(),
        yomoku_core::document::LibraryDocument {
            title: title.into(),
            ..summary
        }
    );
    assert_eq!(
        storage.load_paste(survivor.document_id).unwrap(),
        Some(survivor)
    );
    drop(storage);
    let bytes = fs::read(database.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.load_document_reader_snapshot(id).unwrap(),
        Some(expected)
    );
    assert!(storage.rename_document(id, title).unwrap());
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    let PasteImportOutcome::Existing(existing) = storage
        .import_analyzed_paste(&text, 900, |_| -> Result<_, ()> {
            panic!("duplicate must not analyze")
        })
        .unwrap()
    else {
        panic!("existing")
    };
    assert_eq!(existing.document_id, id);
    assert_eq!(existing.title, title);
    assert_eq!(retained_rows(&storage), before);
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
}

#[test]
fn document_rename_file_formats_preserve_original_bytes_assets_and_live_duplicate_titles() {
    for kind in ["txt", "srt", "epub"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let import = |storage: &mut Storage| match kind {
            "txt" => match storage
                .import_analyzed_txt(
                    "｜猫《ねこ》 犬".as_bytes(),
                    "原本.txt",
                    Some(TextEncoding::Utf8),
                    100,
                    fixture,
                )
                .unwrap()
            {
                TxtImportOutcome::Created(v) => v.document.document_id,
                TxtImportOutcome::Existing(v) => v.document_id,
            },
            "srt" => match storage
                .import_analyzed_srt(
                    b"1\n00:00:01,000 --> 00:00:02,000\n\xe7\x8c\xab\n",
                    "原本.srt",
                    Some(TextEncoding::Utf8),
                    100,
                    fixture,
                )
                .unwrap()
            {
                SrtImportOutcome::Created(v) => v.document.document_id,
                SrtImportOutcome::Existing(v) => v.document_id,
            },
            "epub" => match storage
                .import_analyzed_epub(
                    &epub_fixture::book(true, "<p>猫 犬</p>"),
                    "原本.epub",
                    100,
                    fixture,
                )
                .unwrap()
            {
                EpubImportOutcome::Created(v) => v.document.document_id,
                EpubImportOutcome::Existing(v) => v.document_id,
            },
            _ => unreachable!(),
        };
        let id = import(&mut storage);
        let mut snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        // Capture recency after the explicit open too.
        let after_open = retained_rows(&storage);
        assert!(storage.rename_document(id, "改題𠮷").unwrap());
        assert_eq!(retained_rows(&storage), after_open);
        snapshot.document.title = "改題𠮷".into();
        assert_eq!(
            storage.load_document_reader_snapshot(id).unwrap(),
            Some(snapshot)
        );
        assert_eq!(import(&mut storage), id);
        assert_eq!(storage.list_documents().unwrap()[0].title, "改題𠮷");
        assert_eq!(retained_rows(&storage), after_open);
    }
}

#[test]
fn document_rename_rejects_blank_and_missing_ids_and_rolls_back_statement_or_commit_failures() {
    for trigger in [
        "CREATE TRIGGER fail BEFORE UPDATE OF title ON documents BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER UPDATE OF title ON documents BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let document = storage.unanalyzed_fixture(&PasteText::new("原本\n本文".into()).unwrap()).unwrap();
        for blank in ["", " \n\t\u{3000}"] {
            assert!(matches!(storage.rename_document(document.document_id, blank), Err(StorageError::InvalidDocument)));
        }
        assert!(!storage.rename_document(999.try_into().unwrap(), "不在").unwrap());
        storage.connection.execute_batch(trigger).unwrap();
        let before = retained_rows(&storage);
        assert!(storage.rename_document(document.document_id, "改題").is_err());
        assert_eq!(retained_rows(&storage), before);
        assert_eq!(storage.load_paste(document.document_id).unwrap(), Some(document.clone()));
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.load_paste(document.document_id).unwrap(), Some(document.clone()));
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        assert!(storage.rename_document(document.document_id, "改題").unwrap());
        storage.delete_document(document.document_id).unwrap();
        assert!(!storage.rename_document(document.document_id, "復活しない").unwrap());
    }
}
