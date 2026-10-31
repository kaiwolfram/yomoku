use std::{convert::Infallible, sync::mpsc, time::Duration};

use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::{ChapterText, DocumentId, PasteText, TextOffset},
    pagination::{LogicalPages, PageLength, ReadingPosition},
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_import::epub::{self, EpubChapter};

use super::{imports::fixture, *};

use super::epub_fixture::book;

fn bytes() -> Vec<u8> {
    book(true, "<p>猫 猫 犬 English</p>")
}

fn created(outcome: EpubImportOutcome) -> AnalyzedEpub {
    match outcome {
        EpubImportOutcome::Created(result) => *result,
        _ => panic!("expected new EPUB"),
    }
}

fn no_analysis(_: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    panic!("must skip analysis")
}

fn content(chapter: &StoredEpubChapter) -> EpubChapter {
    EpubChapter {
        path: chapter.path.clone(),
        text: chapter.chapter.text.clone(),
        ruby: chapter.ruby.clone(),
        spans: chapter.spans.clone(),
        images: chapter.images.clone(),
    }
}

fn totals(storage: &Storage) -> Vec<i64> {
    [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "imported_files",
        "epub_documents",
        "epub_chapters",
        "epub_assets",
        "epub_spans",
        "epub_inline_images",
        "source_ruby",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_positions",
        "reading_completions",
        "reading_occurrences",
    ]
    .map(|table| {
        storage
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    })
    .to_vec()
}

fn sequences(storage: &Storage) -> Vec<(String, i64)> {
    storage
        .connection
        .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn epub_chapters_assets_metadata_and_original_analysis_reopen_read_only() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let observer = Storage::open(database.path()).unwrap();
    let bytes = bytes();
    let parsed = epub::parse(&bytes).unwrap();
    let mut calls = 0;
    let imported = created(
        storage
            .import_analyzed_epub(&bytes, "原本.epub", 123, |chapter| {
                assert_eq!(totals(&observer), vec![0; 18]);
                assert_eq!(chapter.text, parsed.chapters[calls].text);
                calls += 1;
                fixture(chapter)
            })
            .unwrap(),
    );
    assert_eq!(calls, 2);
    assert_eq!(imported.document.title, "日本語の本 & EPUB");
    assert_eq!(imported.document.filename, "原本.epub");
    assert_eq!(imported.document.original_bytes, bytes);
    assert_eq!(imported.document.metadata_title, parsed.title);
    assert_eq!(imported.document.cover, parsed.cover);
    assert_eq!(imported.document.images, parsed.images);
    assert_eq!(
        imported
            .document
            .chapters
            .iter()
            .map(content)
            .collect::<Vec<_>>(),
        parsed.chapters
    );
    assert!(imported.document.reading_position.is_none());
    assert!(!imported.document.finished_at_least_once);
    let mut surfaces = std::collections::BTreeSet::new();
    for (chapter, id) in imported
        .document
        .chapters
        .iter()
        .zip(&imported.analysis_ids)
    {
        let expected = fixture(&chapter.chapter).unwrap();
        assert_eq!(
            storage.chapter_analysis(*id).unwrap(),
            Some(expected.clone())
        );
        surfaces.extend(
            expected
                .tokens
                .into_iter()
                .filter(|token| token.is_japanese)
                .map(|token| token.surface),
        );
    }
    assert_eq!(imported.registered_surfaces, surfaces.len());
    assert_eq!(totals(&storage)[15..], [0, 0, 0]);
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage
            .load_epub_document(imported.document.document_id)
            .unwrap(),
        Some(imported.document.clone())
    );
    assert_eq!(
        storage.find_imported_file_id(&bytes).unwrap(),
        Some(imported.document.document_id)
    );
    assert_eq!(fs::read(database.path()).unwrap(), before);
    // The single-text reader must fail rather than silently discard chapters.
    assert!(matches!(
        storage.load_paste(imported.document.document_id),
        Err(StorageError::InvalidDocument)
    ));
}

#[test]
fn epub_missing_title_uses_literal_filename_and_image_only_chapter_is_retained() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = book(false, r#"<p><img src="../images/cover.png" alt="猫"/></p>"#);
    let imported = created(
        storage
            .import_analyzed_epub(&bytes, "  表紙.epub  ", 1, fixture)
            .unwrap(),
    );
    assert_eq!(imported.document.title, "  表紙.epub  ");
    assert_eq!(imported.document.metadata_title, None);
    assert_eq!(imported.document.chapters[1].chapter.text, "\u{fffc}");
    assert_eq!(imported.document.chapters[1].images[0].alt, "猫");
    let chapters: Vec<_> = imported
        .document
        .chapters
        .iter()
        .map(|c| c.chapter.clone())
        .collect();
    for preset in PageLength::ALL {
        let pages = LogicalPages::new(&chapters, preset).unwrap();
        assert_eq!(pages.pages().len(), 2);
        assert_ne!(pages.pages()[0].chapter_id, pages.pages()[1].chapter_id);
    }
    assert_eq!(
        storage
            .load_epub_document(imported.document.document_id)
            .unwrap(),
        Some(imported.document)
    );
}

#[test]
fn epub_duplicates_preserve_live_state_skip_parser_and_share_original_file_identity() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = bytes();
    let imported = created(
        storage
            .import_analyzed_epub(&bytes, "first.epub", 1, fixture)
            .unwrap(),
    );
    let id = imported.document.document_id;
    let chapter = &imported.document.chapters[1].chapter;
    let position = ReadingPosition {
        source_id: chapter.source_id,
        chapter_id: chapter.chapter_id,
        offset: TextOffset::at(&chapter.text, "猫 ".len()).unwrap(),
    };
    crate::documents::write_position_in(&storage.connection, id, position).unwrap();
    storage
        .connection
        .execute(
            "UPDATE documents SET title = 'renamed', finished_at_least_once = 1",
            [],
        )
        .unwrap();
    let before = storage.load_epub_document(id).unwrap();
    let reference = storage.list_documents().unwrap().remove(0);
    let counts = totals(&storage);
    let seq = sequences(&storage);
    for filename in ["other.epub", "", "other.txt"] {
        assert!(
            matches!(storage.import_analyzed_epub(&bytes, filename, 2, no_analysis).unwrap(), EpubImportOutcome::Existing(existing) if existing == reference)
        );
        assert!(
            matches!(storage.import_analyzed_txt(&bytes, filename, None, 2, no_analysis).unwrap(), TxtImportOutcome::Existing(existing) if existing == reference)
        );
        assert!(
            matches!(storage.import_analyzed_srt(&bytes, filename, None, 2, no_analysis).unwrap(), crate::SrtImportOutcome::Existing(existing) if existing == reference)
        );
    }
    // Reference lookup itself remains read-only even for a finished book.
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage.find_imported_file(&bytes).unwrap(),
        Some(reference.clone())
    );
    assert_eq!(storage.find_imported_txt(&bytes).unwrap(), Some(reference));
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    assert_eq!(storage.load_epub_document(id).unwrap(), before);
    assert_eq!(totals(&storage), counts);
    assert_eq!(sequences(&storage), seq);
    // A different first interpretation is referenced before EPUB validation.
    let TxtImportOutcome::Created(txt) = storage
        .import_analyzed_txt(b"English", "first.txt", None, 3, fixture)
        .unwrap()
    else {
        panic!()
    };
    assert!(
        matches!(storage.import_analyzed_epub(b"English", "", 4, no_analysis).unwrap(), EpubImportOutcome::Existing(existing) if existing == txt.document.library_summary().unwrap())
    );
    let paste = storage
        .create_paste(&PasteText::new("English".into()).unwrap())
        .unwrap();
    assert_ne!(paste.document_id, txt.document.document_id);
    let different_bytes = book(true, "<p>猫 猫 犬 English</p><!--same text-->");
    let different = created(
        storage
            .import_analyzed_epub(&different_bytes, "same.epub", 5, fixture)
            .unwrap(),
    );
    assert_eq!(
        content(&different.document.chapters[1]),
        content(&imported.document.chapters[1])
    );
    assert_ne!(different.document.document_id, id);
}

#[test]
fn epub_unique_registration_across_chapters_preserves_all_five_statuses() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let mut entries = Vec::new();
    for (word, status) in [
        ("猫", VocabularyStatus::New),
        ("犬", VocabularyStatus::Learning),
        ("鳥", VocabularyStatus::Familiar),
        ("魚", VocabularyStatus::Known),
        ("馬", VocabularyStatus::Ignored),
    ] {
        let surface = Surface::new(word).unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&surface), 1)
            .unwrap();
        let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
        entries.push(
            storage
                .set_vocabulary_status(entry.id, status, 2)
                .unwrap()
                .unwrap(),
        );
    }
    let bytes = book(true, "<p>猫 猫 犬 鳥 魚 馬 𠮷</p>");
    let imported = created(
        storage
            .import_analyzed_epub(&bytes, "words.epub", 3, fixture)
            .unwrap(),
    );
    let mut new = std::collections::BTreeSet::new();
    for chapter in &imported.document.chapters {
        new.extend(
            fixture(&chapter.chapter)
                .unwrap()
                .tokens
                .into_iter()
                .filter(|t| t.is_japanese)
                .map(|t| t.surface),
        );
    }
    assert_eq!(imported.registered_surfaces, new.len() - 5);
    for entry in entries {
        assert_eq!(
            storage.vocabulary_entry(&entry.surface).unwrap(),
            Some(entry)
        );
    }
    assert_eq!(totals(&storage)[15..], [0, 0, 0]);
}

#[test]
fn epub_analysis_failure_in_later_chapter_and_invalid_input_discard_only_current_file() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .create_analyzed_paste(&PasteText::new("保存".into()).unwrap(), 1, fixture)
        .unwrap();
    let before = totals(&storage);
    let seq = sequences(&storage);
    let bytes = bytes();
    let mut calls = 0;
    assert!(matches!(
        storage.import_analyzed_epub(&bytes, "failed.epub", 2, |chapter| {
            calls += 1;
            if calls == 2 {
                Err("second chapter failed")
            } else {
                Ok(fixture(chapter).unwrap())
            }
        }),
        Err(EpubImportError::Analysis("second chapter failed"))
    ));
    assert_eq!(calls, 2);
    for invalid in 0..3 {
        let mut calls = 0;
        assert!(matches!(
            storage.import_analyzed_epub(&bytes, "invalid.epub", 2, |chapter| {
                calls += 1;
                let mut analysis = fixture(chapter).unwrap();
                if calls == 2 {
                    match invalid {
                        0 => analysis.mode = SplitMode::C,
                        1 => analysis.tokens.clear(),
                        _ => analysis.tokens[0].surface = "wrong".into(),
                    }
                }
                Ok::<_, Infallible>(analysis)
            }),
            Err(EpubImportError::Storage(StorageError::InvalidAnalysis))
        ));
    }
    assert!(matches!(
        storage.import_analyzed_epub(b"invalid", "bad.epub", 2, no_analysis),
        Err(EpubImportError::Parse(_))
    ));
    assert!(matches!(
        storage.import_analyzed_epub(&bytes, " ", 2, no_analysis),
        Err(EpubImportError::Storage(StorageError::InvalidDocument))
    ));
    assert!(matches!(
        storage.import_analyzed_epub(&bytes, "bad.epub", -1, no_analysis),
        Err(EpubImportError::Storage(StorageError::InvalidVocabulary))
    ));
    assert_eq!(totals(&storage), before);
    assert_eq!(sequences(&storage), seq);
    created(
        storage
            .import_analyzed_epub(&bytes, "retry.epub", 3, fixture)
            .unwrap(),
    );
}

#[test]
fn epub_every_write_and_deferred_commit_failure_roll_back_all_chapters_assets_and_ids() {
    for table in [
        "sources",
        "documents",
        "imported_files",
        "epub_documents",
        "epub_assets",
        "chapters",
        "chapter_texts",
        "epub_chapters",
        "source_ruby",
        "epub_spans",
        "epub_inline_images",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "commit",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage
            .create_analyzed_paste(&PasteText::new("保存".into()).unwrap(), 1, fixture)
            .unwrap();
        let before = totals(&storage);
        let seq = sequences(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TEMP TRIGGER fail_import AFTER INSERT ON vocabulary_status_events BEGIN INSERT INTO child VALUES (99); END;").unwrap();
        } else {
            storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fail_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        assert!(
            matches!(
                storage.import_analyzed_epub(&bytes(), "failed.epub", 2, fixture),
                Err(EpubImportError::Storage(StorageError::Sqlite(_)))
            ),
            "{table}"
        );
        assert_eq!(totals(&storage), before, "{table}");
        assert_eq!(sequences(&storage), seq, "{table}");
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_import")
            .unwrap();
        let retried = created(
            storage
                .import_analyzed_epub(&bytes(), "retry.epub", 3, fixture)
                .unwrap(),
        );
        drop(storage);
        assert_eq!(
            Storage::open(database.path())
                .unwrap()
                .load_epub_document(retried.document.document_id)
                .unwrap(),
            Some(retried.document)
        );
    }
}

#[test]
fn epub_cancel_during_second_analysis_rolls_back_first_chapter_and_preserves_prior_files() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .create_analyzed_paste(&PasteText::new("保存".into()).unwrap(), 1, fixture)
        .unwrap();
    let before = totals(&storage);
    let seq = sequences(&storage);
    let cancellation = ImportCancellation::default();
    let caller = cancellation.clone();
    let path = database.path();
    let (entered, ready) = mpsc::channel();
    let (release, finish) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut storage = Storage::open(path).unwrap();
        let mut calls = 0;
        storage
            .import_analyzed_epub_cancellable(
                &bytes(),
                "cancel.epub",
                2,
                &cancellation,
                |chapter| {
                    calls += 1;
                    if calls == 2 {
                        entered.send(()).unwrap();
                        finish.recv_timeout(Duration::from_secs(10)).unwrap();
                    }
                    fixture(chapter)
                },
            )
            .unwrap()
    });
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(totals(&storage), before);
    caller.cancel();
    release.send(()).unwrap();
    assert!(worker.join().unwrap().is_none());
    assert_eq!(totals(&storage), before);
    assert_eq!(sequences(&storage), seq);
    let late = ImportCancellation::default();
    let saved = storage
        .import_analyzed_epub_cancellable(&bytes(), "retry.epub", 3, &late, fixture)
        .unwrap()
        .unwrap();
    late.cancel();
    let saved = created(saved);
    assert!(storage
        .load_epub_document(saved.document.document_id)
        .unwrap()
        .is_some());
}

#[test]
fn epub_pre_cancelled_attempt_never_touches_storage_or_parser() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cancellation = ImportCancellation::default();
    cancellation.cancel();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert!(storage
        .import_analyzed_epub_cancellable(b"invalid", "", -1, &cancellation, no_analysis)
        .unwrap()
        .is_none());
}

#[test]
fn epub_concurrent_creators_commit_one_complete_book() {
    let database = TestDatabase::new();
    Storage::open(database.path()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let workers: Vec<_> = (0..2)
        .map(|index| {
            let path = database.path();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage
                    .import_analyzed_epub(&bytes(), &format!("{index}.epub"), 1, fixture)
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let mut creates = 0;
    let ids: Vec<_> = workers
        .into_iter()
        .map(|worker| match worker.join().unwrap() {
            EpubImportOutcome::Created(imported) => {
                creates += 1;
                imported.document.document_id
            }
            EpubImportOutcome::Existing(document) => document.document_id,
        })
        .collect();
    assert_eq!(creates, 1);
    assert_eq!(ids[0], ids[1]);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage
            .load_epub_document(ids[0])
            .unwrap()
            .unwrap()
            .chapters
            .len(),
        2
    );
}

#[test]
fn epub_corrupt_saved_content_assets_identity_or_ranges_fail_without_repair() {
    for corruption in [
        "UPDATE imported_files SET original_bytes = X'00'",
        "DELETE FROM epub_documents",
        "DELETE FROM epub_spans WHERE span_index = 0",
        "UPDATE epub_spans SET start_byte = 1 WHERE span_index = 0",
        "UPDATE epub_spans SET span_index = span_index + 10",
        "DELETE FROM source_ruby",
        "UPDATE source_ruby SET start_byte = start_byte + 1",
        "DELETE FROM epub_inline_images",
        "UPDATE epub_inline_images SET start_byte = 0, end_byte = 3",
        "UPDATE epub_assets SET bytes = X'00'",
        "UPDATE epub_documents SET cover_path = 'missing'",
        "UPDATE epub_chapters SET path = '' WHERE chapter_id = 1",
        "DELETE FROM epub_chapters WHERE chapter_id = 2",
        "UPDATE chapter_texts SET text = 'changed'",
        "UPDATE chapters SET chapter_index = chapter_index + 3",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let bytes = bytes();
        let imported = created(
            storage
                .import_analyzed_epub(&bytes, "bad.epub", 1, fixture)
                .unwrap(),
        );
        storage.connection.execute_batch(corruption).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(
            matches!(
                storage.load_epub_document(imported.document.document_id),
                Err(StorageError::InvalidDocument)
            ),
            "{corruption}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert!(
            matches!(
                storage.find_imported_file(&bytes),
                Err(StorageError::InvalidDocument)
            ),
            "reference: {corruption}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before);
        if corruption.contains("original_bytes") {
            assert!(matches!(
                storage.find_imported_file_id(&bytes),
                Err(StorageError::InvalidDocument)
            ));
        }
    }
}

#[test]
fn epub_live_deletion_discards_assets_and_content_but_preserves_provenance_and_statuses() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = bytes();
    let old = created(
        storage
            .import_analyzed_epub(&bytes, "old.epub", 1, fixture)
            .unwrap(),
    );
    let surface = Surface::new("猫").unwrap();
    let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 2)
        .unwrap();
    storage
        .connection
        .execute("UPDATE documents SET finished_at_least_once = 1", [])
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(old.document.document_id)],
        )
        .unwrap();
    for chapter in &old.document.chapters {
        storage
            .connection
            .execute(
                "DELETE FROM chapter_texts WHERE chapter_id = ?1",
                [i64::from(chapter.chapter.chapter_id)],
            )
            .unwrap();
    }
    assert!(storage
        .load_epub_document(old.document.document_id)
        .unwrap()
        .is_none());
    assert_eq!(totals(&storage)[4..11], [0; 7]);
    let new = created(
        storage
            .import_analyzed_epub(&bytes, "new.epub", 3, fixture)
            .unwrap(),
    );
    assert_ne!(new.document.document_id, old.document.document_id);
    assert_ne!(new.document.source_id, old.document.source_id);
    assert_eq!(new.registered_surfaces, 0);
    assert!(!new.document.finished_at_least_once);
    assert!(new.document.reading_position.is_none());
    assert_eq!(
        storage.vocabulary_entry(&surface).unwrap().unwrap().status,
        VocabularyStatus::Known
    );
    assert!(storage
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sources WHERE id = ?1)",
            [i64::from(old.document.source_id)],
            |row| row.get::<_, bool>(0)
        )
        .unwrap());
}

#[test]
fn epub_migration_preserves_existing_files_history_foreign_keys_and_sequence_after_failure() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&mut connection, &MIGRATIONS[..13]).unwrap();
    let mut storage = Storage { connection };
    let TxtImportOutcome::Created(txt) = storage
        .import_analyzed_txt(
            "｜猫《ねこ》".as_bytes(),
            "old.txt",
            Some(yomoku_import::txt::TextEncoding::Utf8),
            1,
            fixture,
        )
        .unwrap()
    else {
        panic!()
    };
    let SrtImportOutcome::Created(srt) = storage
        .import_analyzed_srt(b"English", "old.srt", None, 1, fixture)
        .unwrap()
    else {
        panic!()
    };
    let source = storage.load_txt_source(txt.document.document_id).unwrap();
    let cues = storage.load_srt_source(srt.document.document_id).unwrap();
    storage.connection.execute_batch("INSERT INTO sources (id, kind, original_title) VALUES (100, 'paste', 'deleted'); DELETE FROM sources WHERE id = 100; CREATE TABLE epub_spans (fixture TEXT)").unwrap();
    let seq = sequences(&storage);
    assert!(matches!(
        migrate(&mut storage.connection, MIGRATIONS),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(storage.schema_version().unwrap(), 13);
    assert_eq!(sequences(&storage), seq);
    assert_eq!(
        storage.load_txt_source(txt.document.document_id).unwrap(),
        source
    );
    assert_eq!(
        storage.load_srt_source(srt.document.document_id).unwrap(),
        cues
    );
    assert!(storage
        .connection
        .execute(
            "INSERT INTO sources (kind, original_title) VALUES ('epub', 'failed')",
            []
        )
        .is_err());
    assert!(storage
        .connection
        .execute(
            "INSERT INTO chapters (source_id, chapter_index) VALUES (999, 0)",
            []
        )
        .is_err());
    storage
        .connection
        .execute_batch("DROP TABLE epub_spans")
        .unwrap();
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 23);
    assert_eq!(
        storage.load_txt_source(txt.document.document_id).unwrap(),
        source
    );
    assert_eq!(
        storage.load_srt_source(srt.document.document_id).unwrap(),
        cues
    );
    let new = created(
        storage
            .import_analyzed_epub(&bytes(), "new.epub", 2, fixture)
            .unwrap(),
    );
    assert!(i64::from(new.document.source_id) > 100);
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

#[test]
fn epub_library_progress_counts_preceding_chapters_and_rejects_foreign_positions() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = created(
        storage
            .import_analyzed_epub(&bytes(), "progress.epub", 1, fixture)
            .unwrap(),
    );
    let id = imported.document.document_id;
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    let total: usize = imported
        .document
        .chapters
        .iter()
        .map(|c| c.chapter.text.len())
        .sum();
    let mut preceding = 0;
    for chapter in &imported.document.chapters {
        for offset in [0, "猫".len(), chapter.chapter.text.len(), 0] {
            let offset = TextOffset::at(&chapter.chapter.text, offset).unwrap();
            let position = ReadingPosition {
                source_id: chapter.chapter.source_id,
                chapter_id: chapter.chapter.chapter_id,
                offset,
            };
            crate::documents::write_position_in(&storage.connection, id, position).unwrap();
            assert_eq!(
                storage.list_documents().unwrap()[0].progress_percent,
                ((preceding + i64::from(offset) as usize) * 100 / total) as u64
            );
            assert_eq!(
                storage
                    .load_epub_document(id)
                    .unwrap()
                    .unwrap()
                    .reading_position,
                Some(position)
            );
        }
        preceding += chapter.chapter.text.len();
    }
    let foreign = storage
        .create_paste(&PasteText::new("other".into()).unwrap())
        .unwrap();
    crate::documents::write_position_in(
        &storage.connection,
        id,
        ReadingPosition {
            source_id: foreign.chapter.source_id,
            chapter_id: foreign.chapter.chapter_id,
            offset: TextOffset::START,
        },
    )
    .unwrap();
    assert!(matches!(
        storage.list_documents(),
        Err(StorageError::InvalidDocument)
    ));
    assert!(matches!(
        storage.load_epub_document(id),
        Err(StorageError::InvalidDocument)
    ));
}

#[test]
fn epub_preserves_repeated_placements_and_case_insensitive_external_link_scheme() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = book(
        true,
        r##"<p><img src="../images/cover.png"/><img src="../images/cover.png"/></p><p><a href="HTTPS://example.org/">猫</a><a href="#note">犬</a></p>"##,
    );
    let imported = created(
        storage
            .import_analyzed_epub(&bytes, "links.epub", 1, fixture)
            .unwrap(),
    );
    assert_eq!(imported.document.images.len(), 2);
    assert_eq!(imported.document.chapters[1].images.len(), 2);
    assert_eq!(
        storage
            .load_epub_document(imported.document.document_id)
            .unwrap(),
        Some(imported.document)
    );
}

fn image_asset(path: &str) -> yomoku_core::reader_images::ReaderImageAsset {
    yomoku_core::reader_images::ReaderImageAsset {
        path: path.into(),
        media_type: "image/png".into(),
    }
}

#[test]
fn epub_asset_reads_exact_shared_and_cover_bytes_after_read_only_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = created(
        storage
            .import_analyzed_epub(&bytes(), "assets.epub", 1, fixture)
            .unwrap(),
    );
    let id = imported.document.document_id;
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for _ in 0..2 {
        for (path, expected) in &imported.document.images {
            assert_eq!(
                storage
                    .load_epub_image_asset(id, &image_asset(path))
                    .unwrap(),
                Some(expected.clone())
            );
        }
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(totals(&storage)[15..], [0, 0, 0]);
}

#[test]
fn epub_cover_reference_is_metadata_only_read_only_and_content_addressed() {
    use sha2::{Digest, Sha256};
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = created(
        storage
            .import_analyzed_epub(&bytes(), "cover.epub", 1, fixture)
            .unwrap(),
    );
    let id = imported.document.document_id;
    let path = imported.document.cover.unwrap();
    let image = &imported.document.images[&path];
    let expected = EpubCoverReference {
        asset: image_asset(&path),
        sha256: Sha256::digest(&image.bytes).into(),
    };
    // Unrelated BLOB corruption must not trigger full-book integrity reads.
    // The descriptor advertises metadata; the existing asset API verifies bytes.
    storage.connection.execute_batch("UPDATE imported_files SET original_bytes = X'00'; UPDATE chapter_texts SET text = 'damaged'; UPDATE epub_assets SET bytes = X'00'").unwrap();
    drop(storage);
    let before = fs::read(database.path()).unwrap();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            storage.load_epub_cover_reference(id).unwrap(),
            Some(expected.clone())
        );
    }
    assert!(matches!(
        storage.load_epub_image_asset(id, &expected.asset),
        Err(StorageError::InvalidDocument)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(totals(&storage)[15..], [0, 0, 0]);
}

#[test]
fn epub_cover_reference_absence_scopes_and_reimport_do_not_reuse_ids() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = created(
        storage
            .import_analyzed_epub(&bytes(), "cover.epub", 1, fixture)
            .unwrap(),
    );
    let id = first.document.document_id;
    let expected = storage.load_epub_cover_reference(id).unwrap().unwrap();
    let other = created(
        storage
            .import_analyzed_epub(&book(false, "<p>other</p>"), "other.epub", 1, fixture)
            .unwrap(),
    );
    let plain = storage
        .create_paste(&PasteText::new("plain".into()).unwrap())
        .unwrap();
    storage
        .connection
        .execute(
            "UPDATE epub_documents SET cover_path = NULL WHERE document_id = ?1",
            [i64::from(other.document.document_id)],
        )
        .unwrap();
    for absent in [
        other.document.document_id,
        plain.document_id,
        DocumentId::try_from(i64::MAX).unwrap(),
    ] {
        assert_eq!(storage.load_epub_cover_reference(absent).unwrap(), None);
    }
    assert_eq!(
        storage.load_epub_cover_reference(id).unwrap(),
        Some(expected.clone())
    );
    storage
        .connection
        .execute("DELETE FROM documents WHERE id = ?1", [i64::from(id)])
        .unwrap();
    assert_eq!(storage.load_epub_cover_reference(id).unwrap(), None);
    let reimported = created(
        storage
            .import_analyzed_epub(&bytes(), "again.epub", 2, fixture)
            .unwrap(),
    );
    assert_ne!(reimported.document.document_id, id);
    assert_eq!(storage.load_epub_cover_reference(id).unwrap(), None);
    assert_eq!(
        storage
            .load_epub_cover_reference(reimported.document.document_id)
            .unwrap(),
        Some(expected)
    );
}

#[test]
fn epub_cover_reference_rejects_damaged_metadata_without_repairs() {
    for corruption in [
        "DELETE FROM epub_documents",
        "UPDATE imported_files SET text_encoding = 'utf8'",
        "UPDATE epub_documents SET cover_path = 'missing.png'",
        "UPDATE epub_assets SET media_type = 'text/plain'",
        "UPDATE epub_assets SET sha256 = X'00'",
        "UPDATE epub_assets SET bytes = X''",
        "UPDATE epub_assets SET bytes = zeroblob(33554433)",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = created(
            storage
                .import_analyzed_epub(&bytes(), "cover.epub", 1, fixture)
                .unwrap(),
        );
        storage
            .connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        storage.connection.execute_batch(corruption).unwrap();
        storage
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        for _ in 0..2 {
            assert!(
                matches!(
                    storage.load_epub_cover_reference(imported.document.document_id),
                    Err(StorageError::InvalidDocument)
                ),
                "{corruption}"
            );
        }
    }
}

#[test]
fn epub_asset_keys_are_document_scoped_and_deleted_documents_stay_missing() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = created(
        storage
            .import_analyzed_epub(&bytes(), "first.epub", 1, fixture)
            .unwrap(),
    );
    let second = created(
        storage
            .import_analyzed_epub(&book(false, "<p>other</p>"), "second.epub", 1, fixture)
            .unwrap(),
    );
    let plain = storage
        .create_paste(&PasteText::new("plain".into()).unwrap())
        .unwrap();
    let asset = image_asset("OPS/images/挿絵.png");
    // Same key in another book must not supply bytes for the requested book.
    storage
        .connection
        .execute(
            "UPDATE epub_assets SET bytes = X'00' WHERE document_id = ?1 AND path = ?2",
            (i64::from(first.document.document_id), &asset.path),
        )
        .unwrap();
    assert!(matches!(
        storage.load_epub_image_asset(first.document.document_id, &asset),
        Err(StorageError::InvalidDocument)
    ));
    assert!(storage
        .load_epub_image_asset(second.document.document_id, &asset)
        .unwrap()
        .is_some());
    for id in [
        plain.document_id,
        yomoku_core::document::DocumentId::try_from(i64::MAX).unwrap(),
    ] {
        assert!(storage.load_epub_image_asset(id, &asset).unwrap().is_none());
    }
    assert!(storage
        .load_epub_image_asset(second.document.document_id, &image_asset("OPS/missing.png"))
        .unwrap()
        .is_none());
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(first.document.document_id)],
        )
        .unwrap();
    assert!(storage
        .load_epub_image_asset(first.document.document_id, &asset)
        .unwrap()
        .is_none());
    // Reimport allocates a different live identity; old callbacks cannot address it.
    let reimported = created(
        storage
            .import_analyzed_epub(&bytes(), "again.epub", 2, fixture)
            .unwrap(),
    );
    assert_ne!(first.document.document_id, reimported.document.document_id);
    assert!(storage
        .load_epub_image_asset(first.document.document_id, &asset)
        .unwrap()
        .is_none());
    assert!(storage
        .load_epub_image_asset(reimported.document.document_id, &asset)
        .unwrap()
        .is_some());
}

#[test]
fn epub_asset_corruption_invalid_keys_and_media_mismatch_fail_without_writes() {
    for corruption in [
        "UPDATE epub_assets SET bytes = X'00'",
        "UPDATE epub_assets SET sha256 = zeroblob(32)",
        "UPDATE epub_assets SET media_type = 'image/jpeg'",
        "UPDATE imported_files SET text_encoding = 'utf8'",
        "DELETE FROM epub_documents",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = created(
            storage
                .import_analyzed_epub(&bytes(), "bad.epub", 1, fixture)
                .unwrap(),
        );
        storage.connection.execute_batch(corruption).unwrap();
        let before = fs::read(database.path()).unwrap();
        assert!(
            matches!(
                storage.load_epub_image_asset(
                    imported.document.document_id,
                    &image_asset("OPS/images/挿絵.png")
                ),
                Err(StorageError::InvalidDocument)
            ),
            "{corruption}"
        );
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let id = created(
        storage
            .import_analyzed_epub(&bytes(), "keys.epub", 1, fixture)
            .unwrap(),
    )
    .document
    .document_id;
    for path in [
        "",
        "/OPS/images/挿絵.png",
        "OPS/../images/挿絵.png",
        "OPS/./images/挿絵.png",
        "file:OPS/images/挿絵.png",
        "OPS\\images\\挿絵.png",
        "OPS/\0.png",
    ] {
        assert!(
            matches!(
                storage.load_epub_image_asset(id, &image_asset(path)),
                Err(StorageError::InvalidDocument)
            ),
            "{path:?}"
        );
    }
    let mut wrong_media = image_asset("OPS/images/挿絵.png");
    wrong_media.media_type = "image/jpeg".into();
    assert!(matches!(
        storage.load_epub_image_asset(id, &wrong_media),
        Err(StorageError::InvalidDocument)
    ));
}

#[test]
fn epub_asset_budget_is_checked_before_copy_and_other_blobs_are_not_loaded() {
    use sha2::{Digest, Sha256};
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let id = created(
        storage
            .import_analyzed_epub(&bytes(), "bounded.epub", 1, fixture)
            .unwrap(),
    )
    .document
    .document_id;
    let asset = image_asset("OPS/images/挿絵.png");
    assert_eq!(
        MAX_EPUB_ASSET_BYTES,
        epub::EpubLimits::default().entry_bytes
    );
    let max_bytes = vec![0; MAX_EPUB_ASSET_BYTES];
    storage
        .connection
        .execute(
            "UPDATE epub_assets SET bytes = ?1, sha256 = ?2 WHERE path = ?3",
            (
                &max_bytes,
                Sha256::digest(&max_bytes).as_slice(),
                &asset.path,
            ),
        )
        .unwrap();
    assert_eq!(
        storage
            .load_epub_image_asset(id, &asset)
            .unwrap()
            .unwrap()
            .bytes,
        max_bytes
    );
    storage
        .connection
        .execute(
            "UPDATE epub_assets SET bytes = zeroblob(?1) WHERE path = ?2",
            (MAX_EPUB_ASSET_BYTES as i64 + 1, &asset.path),
        )
        .unwrap();
    assert!(matches!(
        storage.load_epub_image_asset(id, &asset),
        Err(StorageError::InvalidDocument)
    ));
    let saved = b"fixture image bytes";
    storage
        .connection
        .execute(
            "UPDATE epub_assets SET bytes = ?1, sha256 = ?2 WHERE path = ?3",
            (
                saved.as_slice(),
                Sha256::digest(saved).as_slice(),
                &asset.path,
            ),
        )
        .unwrap();
    // A narrow image read intentionally does not rehash the full source or
    // unrelated assets. Ordinary document loads retain their stronger checks.
    storage.connection.execute_batch("UPDATE imported_files SET original_bytes = zeroblob(1); UPDATE epub_assets SET bytes = zeroblob(33554433) WHERE path = 'OPS/images/cover.png'").unwrap();
    assert_eq!(
        storage
            .load_epub_image_asset(id, &asset)
            .unwrap()
            .unwrap()
            .bytes,
        saved
    );
    assert!(matches!(
        storage.load_epub_document(id),
        Err(StorageError::InvalidDocument)
    ));
}
