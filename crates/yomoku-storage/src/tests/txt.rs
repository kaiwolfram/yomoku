use std::convert::Infallible;

use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::{ChapterText, PasteText, PastedDocument},
    reader::ReaderAnalysis,
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_import::txt::{TextEncoding, TxtError};

use super::{imports::fixture, positions::position, *};

fn created(outcome: TxtImportOutcome) -> AnalyzedPaste {
    match outcome {
        TxtImportOutcome::Created(imported) => imported,
        TxtImportOutcome::Existing(_) => panic!("expected new file"),
    }
}

fn no_analysis(_: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    panic!("existing or invalid file must not be analyzed")
}

fn count(storage: &Storage, table: &str) -> i64 {
    storage
        .connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn totals(storage: &Storage) -> Vec<i64> {
    [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "imported_files",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_positions",
        "reading_completions",
        "reading_occurrences",
        "paste_content_hashes",
    ]
    .map(|table| count(storage, table))
    .to_vec()
}

#[test]
fn txt_cancelled_before_start_skips_validation_database_and_analyzer() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cancellation = ImportCancellation::default();
    cancellation.cancel();
    // Even BEGIN IMMEDIATE would fail here. A cancelled attempt must not need it.
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(storage
        .import_analyzed_txt_cancellable(&[0xff], "", None, -1, &cancellation, no_analysis)
        .unwrap()
        .is_none());
    assert_eq!(totals(&storage), vec![0; 13]);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn txt_cancel_during_analysis_rolls_back_current_file_and_ids_but_keeps_completed_file() {
    use std::{sync::mpsc, time::Duration};

    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let completed = created(
        storage
            .import_analyzed_txt(
                "猫".as_bytes(),
                "completed.txt",
                Some(TextEncoding::Utf8),
                1,
                fixture,
            )
            .unwrap(),
    );
    let surface = Surface::new("猫").unwrap();
    let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 2)
        .unwrap();
    let before = totals(&storage);
    let sequence = |storage: &Storage| {
        storage
            .connection
            .prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
            .unwrap()
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    let before_sequence = sequence(&storage);
    // Open before the writer starts: opening itself checks migrations under a lock.
    let observer = Storage::open(database.path()).unwrap();
    let cancellation = ImportCancellation::default();
    let worker = cancellation.clone();
    let (entered, ready) = mpsc::channel();
    let (release, finish) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let result = storage.import_analyzed_txt_cancellable(
            "｜猫《ねこ》 犬 𠮷".as_bytes(),
            "partial.txt",
            Some(TextEncoding::Utf8),
            3,
            &worker,
            |chapter| {
                // Provisional document/file rows already exist in the writer.
                entered.send(()).unwrap();
                finish.recv_timeout(Duration::from_secs(10)).unwrap();
                fixture(chapter)
            },
        );
        assert!(result.unwrap().is_none());
        storage
    });
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(totals(&observer), before);
    cancellation.cancel();
    release.send(()).unwrap();
    let storage = handle.join().unwrap();
    assert_eq!(totals(&storage), before);
    assert_eq!(sequence(&storage), before_sequence);
    drop(storage);
    drop(observer);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(totals(&storage), before);
    assert_eq!(sequence(&storage), before_sequence);
    assert_eq!(
        storage.load_paste(completed.document.document_id).unwrap(),
        Some(completed.document)
    );
    assert_eq!(
        storage.vocabulary_entry(&surface).unwrap().unwrap().status,
        VocabularyStatus::Known
    );
    assert!(storage
        .find_imported_file("｜猫《ねこ》 犬 𠮷".as_bytes())
        .unwrap()
        .is_none());
    // A fresh, explicit attempt works; the cancelled handle remains cancelled.
    let retried = created(
        storage
            .import_analyzed_txt_cancellable(
                "｜猫《ねこ》 犬 𠮷".as_bytes(),
                "retry.txt",
                Some(TextEncoding::Utf8),
                4,
                &ImportCancellation::default(),
                fixture,
            )
            .unwrap()
            .unwrap(),
    );
    assert_eq!(retried.registered_surfaces, 2);
    assert!(cancellation.is_cancelled());
    assert_eq!(count(&storage, "documents"), 2);
}

#[test]
fn txt_cancel_after_commit_retains_completed_outcome_and_does_not_cancel_other_files() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cancellation = ImportCancellation::default();
    let first = created(
        storage
            .import_analyzed_txt_cancellable(b"first", "first.txt", None, 1, &cancellation, fixture)
            .unwrap()
            .unwrap(),
    );
    cancellation.cancel();
    let before = totals(&storage);
    assert!(storage
        .import_analyzed_txt_cancellable(
            b"first",
            "duplicate.txt",
            None,
            2,
            &cancellation,
            no_analysis,
        )
        .unwrap()
        .is_none());
    assert_eq!(totals(&storage), before);
    match storage
        .import_analyzed_txt_cancellable(
            b"first",
            "duplicate.txt",
            None,
            2,
            &ImportCancellation::default(),
            no_analysis,
        )
        .unwrap()
        .unwrap()
    {
        TxtImportOutcome::Existing(document) => {
            assert_eq!(document, first.document.library_summary().unwrap())
        }
        _ => panic!("expected existing"),
    }
    let second = created(
        storage
            .import_analyzed_txt_cancellable(
                b"second",
                "second.txt",
                None,
                3,
                &ImportCancellation::default(),
                fixture,
            )
            .unwrap()
            .unwrap(),
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    assert_eq!(
        storage.load_paste(first.document.document_id).unwrap(),
        Some(first.document)
    );
    assert_eq!(
        storage.load_paste(second.document.document_id).unwrap(),
        Some(second.document)
    );
}

#[test]
fn txt_cancellable_commit_failure_rolls_back_and_releases_cancel_gate() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch(
            "CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY);
        CREATE TEMP TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TEMP TRIGGER fail_import AFTER INSERT ON vocabulary_status_events
        BEGIN INSERT INTO child VALUES (99); END;",
        )
        .unwrap();
    let cancellation = ImportCancellation::default();
    assert!(matches!(
        storage.import_analyzed_txt_cancellable(
            "猫".as_bytes(),
            "failed.txt",
            Some(TextEncoding::Utf8),
            1,
            &cancellation,
            fixture,
        ),
        Err(TxtImportError::Storage(StorageError::Sqlite(_)))
    ));
    assert_eq!(totals(&storage), vec![0; 13]);
    cancellation.cancel();
    assert!(cancellation.is_cancelled());
    storage
        .connection
        .execute_batch("DROP TRIGGER fail_import")
        .unwrap();
    created(
        storage
            .import_analyzed_txt_cancellable(
                "猫".as_bytes(),
                "retry.txt",
                Some(TextEncoding::Utf8),
                2,
                &ImportCancellation::default(),
                fixture,
            )
            .unwrap()
            .unwrap(),
    );
}

#[test]
fn txt_atomic_canonical_text_digest_analysis_and_filename_survive_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let observer = Storage::open(database.path()).unwrap();
    let bytes =
        "\u{feff}｜𠮷《よし》 猫 猫\r\nか\u{3099} English 。 📖 \0\r第一章\n［＃改ページ］\n終"
            .as_bytes();
    let imported = created(
        storage
            .import_analyzed_txt(bytes, "本.txt", None, 123, |chapter| {
                assert_eq!(totals(&observer), vec![0; 13]);
                assert_eq!(
                    chapter.text,
                    "𠮷 猫 猫\r\nか\u{3099} English 。 📖 \0\r第一章\n\n終"
                );
                fixture(chapter)
            })
            .unwrap(),
    );
    assert_eq!(imported.document.title, "本.txt");
    assert_eq!(imported.registered_surfaces, 5);
    assert_eq!(count(&storage, "chapters"), 1);
    assert_eq!(count(&storage, "reading_positions"), 0);
    assert_eq!(count(&storage, "reading_completions"), 0);
    assert_eq!(count(&storage, "reading_occurrences"), 0);
    assert_eq!(count(&storage, "paste_content_hashes"), 0);
    let kind: (String, String) = storage
        .connection
        .query_row("SELECT kind, original_title FROM sources", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(kind, ("txt".into(), "本.txt".into()));
    let digests: (Vec<u8>, Vec<u8>) = storage
        .connection
        .query_row(
            "SELECT sha256, canonical_text_sha256 FROM imported_files",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(digests.0, Sha256::digest(bytes).as_slice());
    assert_eq!(
        digests.1,
        Sha256::digest(imported.document.chapter.text.as_bytes()).as_slice()
    );
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let before_read = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        storage
            .load_txt_encoding(imported.document.document_id)
            .unwrap(),
        Some(TextEncoding::Utf8)
    );
    assert_eq!(
        storage.find_imported_file(bytes).unwrap(),
        Some(imported.document.library_summary().unwrap())
    );
    assert!(storage.find_imported_file(b"missing").unwrap().is_none());
    let snapshot = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(crate::tests::as_paste(snapshot.document), imported.document);
    assert_eq!(
        snapshot.analyses[0].analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(fixture(&imported.document.chapter).unwrap())
    );
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    assert_eq!(fs::read(database.path()).unwrap(), before_read);
}

#[test]
fn txt_existing_bypasses_decoding_and_preserves_title_progress_analysis_and_status() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    // Both encodings are valid but disagree: initial import explicitly selects UTF-8.
    let bytes = "©".as_bytes();
    let imported = created(
        storage
            .import_analyzed_txt(bytes, "first.txt", Some(TextEncoding::Utf8), 1, fixture)
            .unwrap(),
    );
    let id = imported.document.document_id;
    let entry = storage
        .vocabulary_entry(&Surface::new("©").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Ignored, 2)
        .unwrap();
    let saved = storage
        .save_document_reading_position(id, position(&imported.document, 2))
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap()
        .unwrap();
    storage
        .connection
        .execute(
            "UPDATE documents SET finished_at_least_once = 1, title = 'renamed' WHERE id = ?1",
            [i64::from(id)],
        )
        .unwrap();
    let expected = PastedDocument {
        title: "renamed".into(),
        finished_at_least_once: true,
        read_override: Some(false),
        ..saved
    };
    assert!(storage.set_document_read(id, false).unwrap());
    let before = totals(&storage);
    // An absent or different selection must not reinterpret an existing file.
    for selection in [None, Some(TextEncoding::ShiftJis)] {
        match storage
            .import_analyzed_txt(bytes, "second.txt", selection, 500, no_analysis)
            .unwrap()
        {
            TxtImportOutcome::Existing(document) => {
                assert_eq!(document, expected.library_summary().unwrap())
            }
            _ => panic!("expected existing"),
        }
    }
    assert_eq!(totals(&storage), before);
    assert_eq!(
        storage
            .vocabulary_entry(&entry.surface)
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Ignored
    );
    assert_eq!(
        storage.load_txt_encoding(id).unwrap().unwrap(),
        TextEncoding::Utf8
    );
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(fixture(&expected.chapter).unwrap())
    );
}

#[test]
fn txt_identity_uses_original_bytes_not_filename_encoding_or_canonical_text_or_paste() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    // All four become the same base text and retain distinct input digests.
    for (bytes, selection) in [
        ("猫".as_bytes(), Some(TextEncoding::Utf8)),
        ("\u{feff}猫".as_bytes(), None),
        ("｜猫《ねこ》".as_bytes(), Some(TextEncoding::Utf8)),
        (&[0x94, 0x4c][..], None),
    ] {
        let imported = created(
            storage
                .import_analyzed_txt(bytes, "same.txt", selection, 1, fixture)
                .unwrap(),
        );
        assert_eq!(imported.document.chapter.text, "猫");
        assert_eq!(
            storage.find_imported_file_id(bytes).unwrap(),
            Some(imported.document.document_id)
        );
    }
    let paste = storage
        .import_analyzed_paste(&PasteText::new("猫".into()).unwrap(), 2, fixture)
        .unwrap();
    let PasteImportOutcome::Created(paste) = paste else {
        panic!("paste identity must be independent")
    };
    assert!(storage
        .load_txt_encoding(paste.document.document_id)
        .unwrap()
        .is_none());
    assert_eq!(count(&storage, "documents"), 5);
    assert_eq!(count(&storage, "vocabulary_status_events"), 1);
    assert_eq!(count(&storage, "paste_content_hashes"), 1);
}

#[test]
fn txt_preserves_all_global_statuses_and_registers_new_surfaces_only() {
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
    let imported = created(
        storage
            .import_analyzed_txt(
                "猫 犬 鳥 魚 馬 猫 𠮷".as_bytes(),
                "words.txt",
                Some(TextEncoding::Utf8),
                3,
                fixture,
            )
            .unwrap(),
    );
    assert_eq!(imported.registered_surfaces, 1);
    for entry in entries {
        assert_eq!(
            storage.vocabulary_entry(&entry.surface).unwrap(),
            Some(entry)
        );
    }
}

#[test]
fn txt_invalid_files_analysis_and_dates_leave_completed_files_untouched() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let saved = created(
        storage
            .import_analyzed_txt(b"English", "good.txt", None, 1, fixture)
            .unwrap(),
    );
    let before = totals(&storage);
    for (bytes, selection, expected) in [
        ("©".as_bytes(), None, TxtError::EncodingRequired),
        (
            &[0xff],
            Some(TextEncoding::Utf8),
            TxtError::InvalidEncoding(TextEncoding::Utf8),
        ),
        (&b"\xff\xfeX"[..], None, TxtError::UnsupportedEncoding),
        (&b" \r\n"[..], None, TxtError::EmptyText),
    ] {
        assert!(
            matches!(storage.import_analyzed_txt(bytes, "bad.txt", selection, 1, no_analysis), Err(TxtImportError::Parse(error)) if error == expected)
        );
        assert_eq!(totals(&storage), before);
    }
    assert!(matches!(
        storage.import_analyzed_txt(b"other", "", None, 1, no_analysis),
        Err(TxtImportError::Storage(StorageError::InvalidDocument))
    ));
    assert!(matches!(
        storage.import_analyzed_txt(b"other", "date.txt", None, -1, no_analysis),
        Err(TxtImportError::Storage(StorageError::InvalidVocabulary))
    ));
    assert!(matches!(
        storage.import_analyzed_txt(b"other", "analysis.txt", None, 1, |_| Err::<
            ChapterAnalysis,
            _,
        >("failed")),
        Err(TxtImportError::Analysis("failed"))
    ));
    for invalid in 0..3 {
        assert!(matches!(
            storage.import_analyzed_txt(b"other", "analysis.txt", None, 1, |chapter| {
                let mut analysis = fixture(chapter).unwrap();
                match invalid {
                    0 => analysis.mode = SplitMode::A,
                    1 => analysis.tokens.clear(),
                    _ => analysis.tokens[0].surface = "wrong".into(),
                }
                Ok::<_, Infallible>(analysis)
            }),
            Err(TxtImportError::Storage(StorageError::InvalidAnalysis))
        ));
    }
    assert_eq!(totals(&storage), before);
    assert_eq!(
        storage.load_paste(saved.document.document_id).unwrap(),
        Some(saved.document)
    );
    created(
        storage
            .import_analyzed_txt(b"other", "retry.txt", None, 2, fixture)
            .unwrap(),
    );
}

#[test]
fn txt_every_write_and_commit_failure_rolls_back_and_can_retry_independently() {
    for table in [
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "imported_files",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "commit",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        created(
            storage
                .import_analyzed_txt(b"English", "completed.txt", None, 1, fixture)
                .unwrap(),
        );
        let before = totals(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY);
                CREATE TEMP TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TEMP TRIGGER fail_import AFTER INSERT ON vocabulary_status_events
                BEGIN INSERT INTO child VALUES (99); END;").unwrap();
        } else {
            storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fail_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        let bytes = "｜猫《ねこ》 犬 𠮷".as_bytes();
        assert!(
            matches!(
                storage.import_analyzed_txt(bytes, "new.txt", Some(TextEncoding::Utf8), 2, fixture),
                Err(TxtImportError::Storage(StorageError::Sqlite(_)))
            ),
            "{table}"
        );
        assert_eq!(totals(&storage), before, "{table}");
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_import")
            .unwrap();
        let imported = created(
            storage
                .import_analyzed_txt(bytes, "new.txt", Some(TextEncoding::Utf8), 2, fixture)
                .unwrap(),
        );
        assert_eq!(imported.registered_surfaces, 3);
        drop(storage);
        assert_eq!(
            Storage::open(database.path())
                .unwrap()
                .load_paste(imported.document.document_id)
                .unwrap(),
            Some(imported.document)
        );
    }
}

#[test]
fn txt_concurrent_identical_files_commit_once_and_reference_same_document() {
    let database = TestDatabase::new();
    Storage::open(database.path()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let path = database.path();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage
                    .import_analyzed_txt(
                        "猫 猫 𠮷".as_bytes(),
                        &format!("{index}.txt"),
                        Some(TextEncoding::Utf8),
                        1,
                        fixture,
                    )
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let mut creates = 0;
    let documents: Vec<_> = handles
        .into_iter()
        .map(|handle| match handle.join().unwrap() {
            TxtImportOutcome::Created(imported) => {
                creates += 1;
                imported.document.library_summary().unwrap()
            }
            TxtImportOutcome::Existing(document) => document,
        })
        .collect();
    assert_eq!(creates, 1);
    assert_eq!(documents[0], documents[1]);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(count(&storage, "documents"), 1);
    assert_eq!(count(&storage, "vocabulary_status_events"), 2);
}

#[test]
fn txt_delete_reimport_resets_live_identity_but_preserves_global_vocabulary_and_provenance() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = "｜猫《ねこ》".as_bytes();
    let imported = created(
        storage
            .import_analyzed_txt(bytes, "old.txt", Some(TextEncoding::Utf8), 1, fixture)
            .unwrap(),
    );
    let document = imported.document;
    let entry = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 2)
        .unwrap();
    // No production deletion API yet; exercise its required live-row boundaries.
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(document.chapter.chapter_id)],
        )
        .unwrap();
    assert_eq!(count(&storage, "imported_files"), 0);
    assert!(storage
        .load_txt_encoding(document.document_id)
        .unwrap()
        .is_none());
    let new = created(
        storage
            .import_analyzed_txt(bytes, "new.txt", Some(TextEncoding::Utf8), 3, fixture)
            .unwrap(),
    );
    assert_ne!(new.document.document_id, document.document_id);
    assert_ne!(new.document.chapter.source_id, document.chapter.source_id);
    assert_eq!(new.registered_surfaces, 0);
    assert!(new.document.reading_position.is_none());
    assert!(!new.document.finished_at_least_once);
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    assert_eq!(count(&storage, "sources"), 2);
    assert_eq!(
        storage
            .vocabulary_entry(&entry.surface)
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Known
    );
}

#[test]
fn txt_encoding_rejects_missing_import_metadata() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let imported = created(
        storage
            .import_analyzed_txt(b"English", "old.txt", None, 1, fixture)
            .unwrap(),
    );
    storage
        .connection
        .execute("DELETE FROM imported_files", [])
        .unwrap();
    assert!(matches!(
        storage.load_txt_encoding(imported.document.document_id),
        Err(StorageError::InvalidDocument)
    ));
}
