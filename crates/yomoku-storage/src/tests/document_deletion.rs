use super::{imports::fixture, *};
use yomoku_core::document::{DocumentId, PasteText};
use yomoku_import::txt::TextEncoding;

fn rows(storage: &Storage, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let mut statement = storage
        .connection
        .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn import_file(storage: &mut Storage, kind: &str) -> DocumentId {
    match kind {
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
            TxtImportOutcome::Created(value) => value.document.document_id,
            _ => panic!("new TXT"),
        },
        "srt" => match storage
            .import_analyzed_srt(
                "1\n00:00:01,000 --> 00:00:02,000\n猫 犬\n".as_bytes(),
                "原本.srt",
                Some(TextEncoding::Utf8),
                100,
                fixture,
            )
            .unwrap()
        {
            SrtImportOutcome::Created(value) => value.document.document_id,
            _ => panic!("new SRT"),
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
            EpubImportOutcome::Created(value) => value.document.document_id,
            _ => panic!("new EPUB"),
        },
        _ => unreachable!(),
    }
}

#[test]
fn asset_delete_checks_foreign_keys_via_the_document_path_index() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut first = None;
    for i in 0..20 {
        let outcome = storage
            .import_analyzed_epub(
                &epub_fixture::book(true, &format!("<p>猫 犬 {i}</p>")),
                "fixture.epub",
                100,
                fixture,
            )
            .unwrap();
        let EpubImportOutcome::Created(value) = outcome else {
            unreachable!()
        };
        first.get_or_insert(value.document.document_id);
    }
    // The cover has no inline reference. Retain the other books' inline rows so
    // the foreign-key probe would have to scan them without the index.
    let id = i64::from(first.unwrap());
    let sql = "DELETE FROM epub_assets WHERE document_id = ?1 AND path = 'OPS/images/cover.png'";
    let measure = |connection: &Connection| {
        connection.execute_batch("SAVEPOINT asset_check").unwrap();
        let (steps, plan) = {
            let mut statement = connection.prepare(sql).unwrap();
            assert_eq!(statement.execute([id]).unwrap(), 1);
            let plan = connection
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .unwrap()
                .query_map([id], |row| row.get::<_, String>(3))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (
                statement.get_status(rusqlite::StatementStatus::VmStep),
                plan,
            )
        };
        connection
            .execute_batch("ROLLBACK TO asset_check; RELEASE asset_check")
            .unwrap();
        (steps, plan)
    };
    let (indexed, plan) = measure(&storage.connection);
    assert!(
        plan.iter().any(|line| line.contains(
            "SEARCH epub_inline_images USING COVERING INDEX epub_inline_images_by_asset"
        )),
        "{plan:?}"
    );
    storage
        .connection
        .execute_batch("DROP INDEX epub_inline_images_by_asset")
        .unwrap();
    let (unindexed, _) = measure(&storage.connection);
    eprintln!("Asset delete SQLite VM steps: {unindexed} -> {indexed}");
    assert!(indexed < unindexed);
}

#[test]
fn document_deletion_cleans_all_file_formats_chapters_assets_and_live_dedup_only() {
    for kind in ["txt", "srt", "epub"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let id = import_file(&mut storage, kind);
        let survivor = storage
            .unanalyzed_fixture(&PasteText::new("残る本文".into()).unwrap())
            .unwrap();
        let original = storage.load_document_reader_snapshot(id).unwrap().unwrap();
        let provenance = rows(&storage, "sources");
        let chapters = rows(&storage, "chapters");
        let vocabulary = rows(&storage, "vocabulary");
        storage.delete_document(id).unwrap();
        let deleted_bytes = fs::read(database.path()).unwrap();
        storage.delete_document(id).unwrap();
        storage
            .delete_document(DocumentId::try_from(i64::MAX).unwrap())
            .unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), deleted_bytes);
        assert_eq!(storage.load_document_reader_snapshot(id).unwrap(), None);
        assert_eq!(
            storage.load_paste(survivor.document_id).unwrap(),
            Some(survivor.clone())
        );
        assert_eq!(rows(&storage, "sources"), provenance);
        assert_eq!(rows(&storage, "chapters"), chapters);
        assert_eq!(rows(&storage, "vocabulary"), vocabulary);
        for table in [
            "imported_files",
            "epub_documents",
            "epub_chapters",
            "epub_spans",
            "epub_assets",
            "epub_inline_images",
            "chapter_analyses",
            "analyzed_tokens",
        ] {
            assert!(rows(&storage, table).is_empty(), "{kind}: {table}");
        }
        assert_eq!(rows(&storage, "chapter_texts").len(), 1);
        assert_eq!(
            original.document.chapters.len(),
            if kind == "epub" { 2 } else { 1 }
        );
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        let new_id = import_file(&mut storage, kind);
        assert!(new_id > id);
        let new = storage
            .load_document_reader_snapshot(new_id)
            .unwrap()
            .unwrap();
        assert!(new.document.reading_position.is_none());
        assert!(!new.document.finished_at_least_once);
        assert_eq!(new.last_read_sequence, None);
        assert_eq!(rows(&storage, "vocabulary"), vocabulary);
        assert!(storage
            .connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none());
    }
}

#[test]
fn document_deletion_rolls_back_text_cascades_entry_and_deferred_commit_then_retries() {
    for trigger in [
        "CREATE TRIGGER fail BEFORE DELETE ON analyzed_tokens BEGIN SELECT RAISE(ABORT, 'tokens'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON chapter_texts BEGIN SELECT RAISE(ABORT, 'text'); END;",
        "CREATE TRIGGER fail BEFORE DELETE ON documents BEGIN SELECT RAISE(ABORT, 'entry'); END;",
        "CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER DELETE ON documents BEGIN INSERT INTO deferred_failure VALUES (-1); END;",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let id = import_file(&mut storage, "epub");
        let before = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        let tables = ["chapter_texts", "chapter_analyses", "analyzed_tokens", "epub_chapters", "epub_assets", "epub_inline_images", "imported_files", "documents", "reading_positions"];
        let rows_before: Vec<_> = tables.iter().map(|table| rows(&storage, table)).collect();
        storage.connection.execute_batch(trigger).unwrap();
        assert!(storage.delete_document(id).is_err());
        assert_eq!(tables.iter().map(|table| rows(&storage, table)).collect::<Vec<_>>(), rows_before);
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.load_document_reader_snapshot(id).unwrap(), Some(before));
        storage.connection.execute_batch("DROP TRIGGER fail").unwrap();
        storage.delete_document(id).unwrap();
        assert!(storage.list_documents().unwrap().is_empty());
    }
}
