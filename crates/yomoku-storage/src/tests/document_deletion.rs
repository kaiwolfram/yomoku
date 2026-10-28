use super::{imports::fixture, *};
use yomoku_core::{
    active_time::PendingReadingTime,
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{DocumentId, PasteText},
    reader::ReaderPage,
    vocabulary::{Surface, VocabularyStatus},
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

#[test]
fn document_deletion_preserves_exact_history_vocabulary_provenance_and_fresh_reimport() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let text = PasteText::new("猫 猫 犬 𠮷 か\u{3099}".into()).unwrap();
    let imported = storage.import_analyzed_paste(&text, 100, fixture).unwrap();
    let PasteImportOutcome::Created(imported) = imported else {
        panic!("new import")
    };
    let id = imported.document.document_id;
    let snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    let saved = storage
        .load_pending_reading_time(id, page.position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(1234).unwrap())
        .unwrap();
    let dog = storage
        .vocabulary_entry(&Surface::new("犬").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(dog.id, VocabularyStatus::Ignored, 200)
        .unwrap();
    let prepared = storage
        .prepare_page_completion(PageCompletionIntent {
            document_id: id,
            page: page.anchor,
            page_length: snapshot.page_length,
            departing_position: page.position,
            completed_at_unix_ms: 500,
            analysis_id: imported.analysis_id,
        })
        .unwrap();
    storage
        .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
        .unwrap();
    // Also leave partial reread time, which deletion must discard.
    storage
        .save_document_reading_position(id, page.position)
        .unwrap();
    let saved = storage
        .load_pending_reading_time(id, page.position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(77).unwrap())
        .unwrap();
    storage
        .connection
        .execute(
            "INSERT INTO custom_dictionary_entries VALUES ('𠮷', 1, 'literal meaning', 'よし')",
            [],
        )
        .unwrap();
    let rule = storage
        .save_segmentation_override(&yomoku_core::analysis::SegmentationOverride {
            scope: yomoku_core::analysis::OverrideScope::AllOccurrences,
            anchor: yomoku_core::document::TextAnchor {
                range: yomoku_core::document::TextRange {
                    start: yomoku_core::document::TextOffset::START,
                    end: yomoku_core::document::TextOffset::at(
                        &imported.document.chapter.text,
                        "猫".len(),
                    )
                    .unwrap(),
                },
                ..page.anchor
            },
            surface: Surface::new("猫").unwrap(),
            parts: vec![Surface::new("猫").unwrap()],
        })
        .unwrap();
    let tables = [
        "sources",
        "chapters",
        "completion_commands",
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "vocabulary",
        "vocabulary_status_events",
        "custom_dictionary_entries",
        "segmentation_overrides",
        "segmentation_override_parts",
        "effective_completion_commands",
        "library_read_clock",
    ];
    let before: Vec<_> = tables.iter().map(|table| rows(&storage, table)).collect();
    storage.delete_document(id).unwrap();
    assert!(storage.list_documents().unwrap().is_empty());
    for table in [
        "documents",
        "chapter_texts",
        "chapter_analyses",
        "analyzed_tokens",
        "paste_content_hashes",
        "reading_positions",
        "pending_reading_time",
    ] {
        assert!(rows(&storage, table).is_empty(), "{table}");
    }
    assert_eq!(
        tables
            .iter()
            .map(|table| rows(&storage, table))
            .collect::<Vec<_>>(),
        before
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert!(storage.segmentation_override(rule).unwrap().unwrap().active);
    let bytes = fs::read(database.path()).unwrap();
    storage.delete_document(id).unwrap();
    storage
        .delete_document(DocumentId::try_from(i64::MAX).unwrap())
        .unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), bytes);
    assert_eq!(
        storage
            .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
            .unwrap(),
        AnalyzedCompletionOutcome::AlreadyRecorded
    );
    let PasteImportOutcome::Created(reimported) =
        storage.import_analyzed_paste(&text, 1000, fixture).unwrap()
    else {
        panic!("reimport must be new")
    };
    assert!(reimported.document.document_id > id);
    assert!(reimported.document.chapter.source_id > imported.document.chapter.source_id);
    assert!(reimported.document.chapter.chapter_id > imported.document.chapter.chapter_id);
    let summary = storage.list_documents().unwrap().pop().unwrap();
    assert_eq!(summary.progress_percent, 0);
    assert!(!summary.finished_at_least_once);
    assert_eq!(summary.last_read_sequence, None);
    let snapshot = storage
        .load_document_reader_snapshot(summary.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        storage
            .load_pending_reading_time(
                summary.document_id,
                snapshot.document.current_position().unwrap()
            )
            .unwrap()
            .pending()
            .milliseconds(),
        0
    );
    assert_eq!(
        tables
            .iter()
            .skip(2)
            .map(|table| rows(&storage, table))
            .collect::<Vec<_>>(),
        before[2..]
    );
    assert!(storage.reverse_segmentation_override(rule).unwrap());
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
