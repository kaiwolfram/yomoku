use super::{imports::fixture, *};
use yomoku_core::{
    active_time::PendingReadingTime,
    completion::PageCompletionIntent,
    document::{DocumentId, PasteText},
    reader::ReaderPage,
};
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
fn document_deletion_cleans_all_file_formats_chapters_assets_and_live_dedup_only() {
    for kind in ["txt", "srt", "epub"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let id = import_file(&mut storage, kind);
        let survivor = storage
            .create_paste(&PasteText::new("残る本文".into()).unwrap())
            .unwrap();
        let original = storage.load_document_reader_snapshot(id).unwrap().unwrap();
        let provenance = rows(&storage, "sources");
        let chapters = rows(&storage, "chapters");
        let vocabulary = rows(&storage, "vocabulary");
        storage.delete_document(id).unwrap();
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
            "source_ruby",
            "subtitle_cues",
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

#[test]
fn bulk_deletion_rolls_back_earlier_documents_on_later_cascade_and_commit_failure() {
    for failure in ["text", "cascade", "document", "commit"] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let first = import_file(&mut storage, "txt");
        let last = import_file(&mut storage, "epub");
        let trigger = match failure {
            "text" => format!("CREATE TRIGGER fail BEFORE DELETE ON chapter_texts WHEN OLD.chapter_id IN (SELECT c.id FROM chapters c JOIN documents d ON d.source_id = c.source_id WHERE d.id = {}) BEGIN SELECT RAISE(ABORT, 'later text'); END;", i64::from(last)),
            "cascade" => format!("CREATE TRIGGER fail BEFORE DELETE ON epub_assets WHEN OLD.document_id = {} BEGIN SELECT RAISE(ABORT, 'later asset'); END;", i64::from(last)),
            "document" => format!("CREATE TRIGGER fail BEFORE DELETE ON documents WHEN OLD.id = {} BEGIN SELECT RAISE(ABORT, 'later document'); END;", i64::from(last)),
            "commit" => format!("CREATE TABLE deferred_failure (id INTEGER REFERENCES documents(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail AFTER DELETE ON documents WHEN OLD.id = {} BEGIN INSERT INTO deferred_failure VALUES (-1); END;", i64::from(last)),
            _ => unreachable!(),
        };
        storage.connection.execute_batch(&trigger).unwrap();
        let names: Vec<String> = storage
            .connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let before: Vec<_> = names.iter().map(|table| rows(&storage, table)).collect();
        assert!(
            storage.delete_documents(&[first, last]).is_err(),
            "{failure}"
        );
        assert_eq!(
            names
                .iter()
                .map(|table| rows(&storage, table))
                .collect::<Vec<_>>(),
            before,
            "{failure}"
        );
        drop(storage);
        let mut storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.list_documents().unwrap().len(), 2);
        storage
            .connection
            .execute_batch("DROP TRIGGER fail")
            .unwrap();
        storage.delete_documents(&[first, last]).unwrap();
        assert!(storage.list_documents().unwrap().is_empty());
    }
}

#[test]
fn bulk_deletion_handles_large_duplicate_and_absent_id_selections_without_parameter_limit() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    // Exercise a selection larger than the older 999-variable SQLite limit.
    let mut ids = Vec::new();
    for n in 0..1100 {
        ids.push(
            storage
                .create_paste(&PasteText::new(format!("本文 {n}")).unwrap())
                .unwrap()
                .document_id,
        );
    }
    let survivor = storage
        .create_paste(&PasteText::new("残る本文".into()).unwrap())
        .unwrap();
    ids.push(ids[0]);
    ids.push(DocumentId::try_from(i64::MAX).unwrap());
    storage.delete_documents(&ids).unwrap();
    assert_eq!(storage.list_documents().unwrap().len(), 1);
    assert_eq!(
        storage.load_paste(survivor.document_id).unwrap(),
        Some(survivor)
    );
}

#[test]
fn bulk_deletion_keeps_history_vocabulary_and_survivor_across_all_formats_and_reimport() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = PasteText::new("猫 猫 犬".into()).unwrap();
    let PasteImportOutcome::Created(pasted) =
        storage.import_analyzed_paste(&text, 1, fixture).unwrap()
    else {
        panic!("paste")
    };
    let mut ids = vec![pasted.document.document_id];
    ids.extend(["txt", "srt", "epub"].map(|kind| import_file(&mut storage, kind)));
    let survivor = storage
        .create_paste(&PasteText::new("残る本文".into()).unwrap())
        .unwrap();
    let survivor_snapshot = storage
        .open_document_reader_snapshot(survivor.document_id)
        .unwrap();
    for &id in &ids {
        let snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
        let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
        let analysis = snapshot
            .analyses
            .iter()
            .find(|value| value.chapter_id == page.anchor.chapter_id)
            .unwrap()
            .analysis;
        let yomoku_core::reader::ReaderAnalysis::Available(analysis_id) = analysis else {
            panic!("B")
        };
        let saved = storage
            .load_pending_reading_time(id, page.position)
            .unwrap();
        storage
            .save_pending_reading_time(saved, PendingReadingTime::from_millis(1234).unwrap())
            .unwrap();
        let prepared = storage
            .prepare_page_completion(PageCompletionIntent {
                document_id: id,
                page: page.anchor,
                page_length: snapshot.page_length,
                departing_position: page.position,
                completed_at_unix_ms: 500,
                analysis_id,
            })
            .unwrap();
        storage
            .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
            .unwrap();
        storage
            .save_document_reading_position(id, page.position)
            .unwrap();
        let saved = storage
            .load_pending_reading_time(id, page.position)
            .unwrap();
        storage
            .save_pending_reading_time(saved, PendingReadingTime::from_millis(77).unwrap())
            .unwrap();
    }
    let tables = [
        "sources",
        "chapters",
        "completion_commands",
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "vocabulary",
        "vocabulary_status_events",
        "effective_completion_commands",
        "library_read_clock",
    ];
    let before: Vec<_> = tables.iter().map(|table| rows(&storage, table)).collect();
    storage.delete_documents(&ids).unwrap();
    assert_eq!(
        storage
            .load_document_reader_snapshot(survivor.document_id)
            .unwrap(),
        survivor_snapshot
    );
    assert_eq!(storage.list_documents().unwrap().len(), 1);
    assert_eq!(
        tables
            .iter()
            .map(|table| rows(&storage, table))
            .collect::<Vec<_>>(),
        before
    );
    for table in [
        "imported_files",
        "source_ruby",
        "subtitle_cues",
        "epub_documents",
        "epub_chapters",
        "epub_spans",
        "epub_assets",
        "epub_inline_images",
        "chapter_analyses",
        "analyzed_tokens",
        "paste_content_hashes",
        "pending_reading_time",
    ] {
        assert!(rows(&storage, table).is_empty(), "{table}");
    }
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = fs::read(database.path()).unwrap();
    storage.delete_documents(&ids).unwrap();
    storage.delete_documents(&[]).unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    let PasteImportOutcome::Created(reimported) =
        storage.import_analyzed_paste(&text, 1000, fixture).unwrap()
    else {
        panic!("fresh paste")
    };
    let mut fresh = vec![reimported.document.document_id];
    fresh.extend(["txt", "srt", "epub"].map(|kind| import_file(&mut storage, kind)));
    assert!(fresh.iter().all(|id| *id > *ids.last().unwrap()));
    for id in fresh {
        let snapshot = storage.load_document_reader_snapshot(id).unwrap().unwrap();
        assert!(snapshot.document.reading_position.is_none());
        assert!(!snapshot.document.finished_at_least_once);
        assert!(snapshot.last_read_sequence.is_none());
    }
    assert_eq!(
        tables
            .iter()
            .skip(2)
            .map(|table| rows(&storage, table))
            .collect::<Vec<_>>(),
        before[2..]
    );
}
