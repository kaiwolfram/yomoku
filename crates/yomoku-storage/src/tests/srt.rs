use std::{convert::Infallible, sync::mpsc, time::Duration};

use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::{ChapterText, PasteText},
    reader::ReaderAnalysis,
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_import::txt::{TextEncoding, TxtError};

use super::{imports::fixture, positions::position, *};

fn created(outcome: SrtImportOutcome) -> AnalyzedPaste {
    match outcome {
        SrtImportOutcome::Created(imported) => imported,
        SrtImportOutcome::Existing(_) => panic!("expected new file"),
    }
}

fn no_analysis(_: &ChapterText) -> Result<ChapterAnalysis, Infallible> {
    panic!("duplicate or rejected file must not be analyzed")
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
fn srt_original_bytes_cues_analysis_and_title_survive_read_only_reopen() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let observer = Storage::open(database.path()).unwrap();
    let bytes = "\u{feff}1\r\n00:00:01,000 --> 00:00:02,000\r\n<i>猫 猫</i>\r\n𠮷 📖\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\n話者: ｜犬《いぬ》［＃改ページ］".as_bytes();
    let canonical = "猫 猫\n𠮷 📖\n\n話者: ｜犬《いぬ》［＃改ページ］";
    let imported = created(
        storage
            .import_analyzed_srt(bytes, "字幕.srt", None, 123, |chapter| {
                assert_eq!(totals(&observer), vec![0; 13]);
                assert_eq!(chapter.text, canonical);
                fixture(chapter)
            })
            .unwrap(),
    );
    assert_eq!(imported.document.title, "字幕.srt");
    assert_eq!(imported.registered_surfaces, 4);
    for table in [
        "reading_positions",
        "reading_completions",
        "reading_occurrences",
        "paste_content_hashes",
    ] {
        assert_eq!(count(&storage, table), 0, "{table}");
    }
    let kind: (String, String) = storage
        .connection
        .query_row("SELECT kind, original_title FROM sources", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(kind, ("srt".into(), "字幕.srt".into()));
    let source = SrtFile {
        original_bytes: bytes.to_vec(),
        encoding: TextEncoding::Utf8,
    };
    drop(storage);
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert_eq!(
        srt_file(&storage.connection, imported.document.document_id).unwrap(),
        Some(source)
    );
    assert_eq!(
        storage.find_imported_file(bytes).unwrap(),
        Some(imported.document.library_summary().unwrap())
    );
    assert!(storage
        .load_txt_source(imported.document.document_id)
        .unwrap()
        .is_none());
    let snapshot = storage
        .load_document_reader_snapshot(imported.document.document_id)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.document, imported.document.clone().into());
    assert_eq!(
        snapshot.analyses[0].analysis,
        ReaderAnalysis::Available(imported.analysis_id)
    );
    assert_eq!(
        storage.chapter_analysis(imported.analysis_id).unwrap(),
        Some(fixture(&imported.document.chapter).unwrap())
    );
    assert_eq!(storage.list_documents().unwrap()[0].progress_percent, 0);
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn srt_duplicate_bypasses_decoding_and_preserves_progress_and_status() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = "©".as_bytes(); // Both valid encodings disagree.
    let imported = created(
        storage
            .import_analyzed_srt(bytes, "first.srt", Some(TextEncoding::Utf8), 1, fixture)
            .unwrap(),
    );
    let entry = storage
        .vocabulary_entry(&Surface::new("©").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Ignored, 2)
        .unwrap();
    let saved = storage
        .save_document_reading_position(
            imported.document.document_id,
            position(&imported.document, bytes.len()),
        )
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap()
        .unwrap();
    let before = totals(&storage);
    for selection in [None, Some(TextEncoding::ShiftJis)] {
        let SrtImportOutcome::Existing(document) = storage
            .import_analyzed_srt(bytes, "", selection, 3, no_analysis)
            .unwrap()
        else {
            panic!("expected duplicate")
        };
        assert_eq!(document, saved.library_summary().unwrap());
        assert_eq!(totals(&storage), before);
    }
    assert_eq!(
        srt_file(&storage.connection, saved.document_id)
            .unwrap()
            .unwrap()
            .encoding,
        TextEncoding::Utf8
    );
    assert_eq!(
        storage
            .vocabulary_entry(&entry.surface)
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Ignored
    );
}

#[test]
fn srt_identity_is_original_bytes_shared_with_txt_and_separate_from_paste() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let first = created(
        storage
            .import_analyzed_srt(b"<i>English</i>", "first.srt", None, 1, fixture)
            .unwrap(),
    );
    let TxtImportOutcome::Existing(duplicate) = storage
        .import_analyzed_txt(b"<i>English</i>", "same.txt", None, 2, no_analysis)
        .unwrap()
    else {
        panic!("shared file hash")
    };
    assert_eq!(duplicate, first.document.library_summary().unwrap());
    let TxtImportOutcome::Created(txt) = storage
        .import_analyzed_txt(b"English", "first.txt", None, 3, fixture)
        .unwrap()
    else {
        panic!("different bytes")
    };
    let SrtImportOutcome::Existing(duplicate) = storage
        .import_analyzed_srt(b"English", "same.srt", None, 4, no_analysis)
        .unwrap()
    else {
        panic!("shared file hash")
    };
    assert_eq!(duplicate, txt.document.library_summary().unwrap());
    let other = created(
        storage
            .import_analyzed_srt(b"\xEF\xBB\xBFEnglish", "bom.srt", None, 5, fixture)
            .unwrap(),
    );
    assert_eq!(other.document.chapter.text, first.document.chapter.text);
    let PasteImportOutcome::Created(_) = storage
        .import_analyzed_paste(&PasteText::new("English".into()).unwrap(), 6, fixture)
        .unwrap()
    else {
        panic!("separate paste identity")
    };
    assert_eq!(count(&storage, "documents"), 4);
    assert!(srt_file(&storage.connection, txt.document.document_id)
        .unwrap()
        .is_none());
}

#[test]
fn srt_decodes_shift_jis_and_requires_explicit_ambiguous_choice() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = b"1\r\n00:00:01,000 --> 00:00:02,000\r\n\x82\xA0";
    assert!(matches!(
        storage.import_analyzed_srt(bytes, "bad.srt", Some(TextEncoding::Utf8), 1, no_analysis),
        Err(SrtImportError::Decode(TxtError::InvalidEncoding(
            TextEncoding::Utf8
        )))
    ));
    let imported = created(
        storage
            .import_analyzed_srt(bytes, "legacy.srt", None, 2, fixture)
            .unwrap(),
    );
    assert_eq!(imported.document.chapter.text, "あ");
    assert_eq!(
        srt_file(&storage.connection, imported.document.document_id)
            .unwrap()
            .unwrap()
            .encoding,
        TextEncoding::ShiftJis
    );
    assert!(matches!(
        storage.import_analyzed_srt("©".as_bytes(), "choice.srt", None, 3, no_analysis),
        Err(SrtImportError::Decode(TxtError::EncodingRequired))
    ));
    let imported = created(
        storage
            .import_analyzed_srt(
                "©".as_bytes(),
                "choice.srt",
                Some(TextEncoding::ShiftJis),
                4,
                fixture,
            )
            .unwrap(),
    );
    assert_eq!(imported.document.chapter.text, "ﾂｩ");
}

#[test]
fn srt_preserves_all_existing_statuses_and_registers_unique_new_only() {
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
            .import_analyzed_srt(
                "猫 犬 鳥\n\n魚 馬 猫 𠮷".as_bytes(),
                "words.srt",
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
    assert_eq!(count(&storage, "reading_occurrences"), 0);
}

#[test]
fn srt_rejected_files_analysis_and_dates_preserve_completed_files() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let saved = created(
        storage
            .import_analyzed_srt(b"English", "saved.srt", None, 1, fixture)
            .unwrap(),
    );
    let before = totals(&storage);
    for bytes in [
        b"".as_slice(),
        b" \r\n",
        b"1\n00:00:01,000 --> 00:00:02,000\n<i></i>",
    ] {
        assert!(matches!(
            storage.import_analyzed_srt(bytes, "empty.srt", None, 2, no_analysis),
            Err(SrtImportError::Empty(_))
        ));
    }
    for bytes in [b"\xFF".as_slice(), b"\xFF\xFEa\0"] {
        assert!(matches!(
            storage.import_analyzed_srt(bytes, "invalid.srt", None, 2, no_analysis),
            Err(SrtImportError::Decode(_))
        ));
    }
    assert!(matches!(
        storage.import_analyzed_srt(b"other", "", None, 2, no_analysis),
        Err(SrtImportError::Storage(StorageError::InvalidDocument))
    ));
    assert!(matches!(
        storage.import_analyzed_srt(b"other", "date.srt", None, -1, no_analysis),
        Err(SrtImportError::Storage(StorageError::InvalidVocabulary))
    ));
    assert!(matches!(
        storage.import_analyzed_srt(b"other", "analysis.srt", None, 2, |_| Err::<
            ChapterAnalysis,
            _,
        >("failed")),
        Err(SrtImportError::Analysis("failed"))
    ));
    for invalid in 0..3 {
        assert!(matches!(
            storage.import_analyzed_srt(b"other", "analysis.srt", None, 2, |chapter| {
                let mut analysis = fixture(chapter).unwrap();
                match invalid {
                    0 => analysis.mode = SplitMode::A,
                    1 => analysis.tokens.clear(),
                    _ => analysis.tokens[0].surface = "wrong".into(),
                }
                Ok::<_, Infallible>(analysis)
            }),
            Err(SrtImportError::Storage(StorageError::InvalidAnalysis))
        ));
    }
    assert_eq!(totals(&storage), before);
    assert_eq!(
        storage.load_paste(saved.document.document_id).unwrap(),
        Some(saved.document)
    );
    created(
        storage
            .import_analyzed_srt(b"other", "next.srt", None, 3, fixture)
            .unwrap(),
    );
}

#[test]
fn srt_each_write_and_commit_failure_rolls_back_all_rows_and_ids() {
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
        let saved = created(
            storage
                .import_analyzed_srt(b"English", "saved.srt", None, 1, fixture)
                .unwrap(),
        );
        let before = totals(&storage);
        let sequences = sequence(&storage);
        if table == "commit" {
            storage.connection.execute_batch("CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY);
                CREATE TEMP TABLE child (id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TEMP TRIGGER fail_import AFTER INSERT ON vocabulary_status_events BEGIN INSERT INTO child VALUES (99); END;").unwrap();
        } else {
            storage.connection.execute_batch(&format!("CREATE TEMP TRIGGER fail_import AFTER INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'fixture'); END;")).unwrap();
        }
        assert!(
            matches!(
                storage.import_analyzed_srt(
                    "猫 猫\n\n𠮷".as_bytes(),
                    "failed.srt",
                    Some(TextEncoding::Utf8),
                    2,
                    fixture
                ),
                Err(SrtImportError::Storage(StorageError::Sqlite(_)))
            ),
            "{table}"
        );
        assert_eq!(totals(&storage), before, "{table}");
        assert_eq!(sequence(&storage), sequences, "{table}");
        assert_eq!(
            storage.load_paste(saved.document.document_id).unwrap(),
            Some(saved.document)
        );
        storage
            .connection
            .execute_batch("DROP TRIGGER fail_import")
            .unwrap();
        let retried = created(
            storage
                .import_analyzed_srt(
                    "猫 猫\n\n𠮷".as_bytes(),
                    "next.srt",
                    Some(TextEncoding::Utf8),
                    3,
                    fixture,
                )
                .unwrap(),
        );
        assert_eq!(retried.registered_surfaces, 2);
        drop(storage);
        assert!(srt_file(
            &Storage::open(database.path()).unwrap().connection,
            retried.document.document_id
        )
        .unwrap()
        .is_some());
    }
}

fn sequence(storage: &Storage) -> Vec<(String, i64)> {
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
fn srt_cancellation_skips_start_and_rolls_back_blocked_analysis_only() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let cancelled = ImportCancellation::default();
    cancelled.cancel();
    storage
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    assert!(storage
        .import_analyzed_srt_cancellable(&[0xff], "", None, -1, &cancelled, no_analysis)
        .unwrap()
        .is_none());
    storage
        .connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    let saved = created(
        storage
            .import_analyzed_srt(b"English", "saved.srt", None, 1, fixture)
            .unwrap(),
    );
    let before = totals(&storage);
    let sequences = sequence(&storage);
    let cancellation = ImportCancellation::default();
    let caller = cancellation.clone();
    let (entered, ready) = mpsc::channel();
    let (release, finish) = mpsc::channel();
    let path = database.path();
    let worker = std::thread::spawn(move || {
        let mut storage = Storage::open(path).unwrap();
        storage
            .import_analyzed_srt_cancellable(
                "猫\n\n𠮷".as_bytes(),
                "cancel.srt",
                Some(TextEncoding::Utf8),
                2,
                &cancellation,
                |chapter| {
                    entered.send(()).unwrap();
                    finish.recv_timeout(Duration::from_secs(10)).unwrap();
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
    assert_eq!(sequence(&storage), sequences);
    assert_eq!(
        storage.load_paste(saved.document.document_id).unwrap(),
        Some(saved.document)
    );
    let new = created(
        storage
            .import_analyzed_srt(
                "猫\n\n𠮷".as_bytes(),
                "retry.srt",
                Some(TextEncoding::Utf8),
                3,
                fixture,
            )
            .unwrap(),
    );
    let late = ImportCancellation::default();
    let result = storage
        .import_analyzed_srt_cancellable(
            "猫\n\n𠮷".as_bytes(),
            "same.srt",
            None,
            4,
            &late,
            no_analysis,
        )
        .unwrap();
    late.cancel();
    assert!(
        matches!(result, Some(SrtImportOutcome::Existing(document)) if document == new.document.library_summary().unwrap())
    );
}

#[test]
fn srt_concurrent_creators_reference_one_committed_document() {
    let database = TestDatabase::new();
    Storage::open(database.path()).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let path = database.path();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut storage = Storage::open(path).unwrap();
                barrier.wait();
                storage
                    .import_analyzed_srt(
                        "猫\n\n猫 𠮷".as_bytes(),
                        &format!("{index}.srt"),
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
            SrtImportOutcome::Created(imported) => {
                creates += 1;
                imported.document.library_summary().unwrap()
            }
            SrtImportOutcome::Existing(document) => document,
        })
        .collect();
    assert_eq!(creates, 1);
    assert_eq!(documents[0], documents[1]);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(count(&storage, "vocabulary_status_events"), 2);
}

#[test]
fn srt_delete_reimport_discards_live_data_and_preserves_provenance_and_status() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let bytes = "猫\n\n𠮷".as_bytes();
    let old = created(
        storage
            .import_analyzed_srt(bytes, "old.srt", Some(TextEncoding::Utf8), 1, fixture)
            .unwrap(),
    );
    let entry = storage
        .vocabulary_entry(&Surface::new("猫").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 2)
        .unwrap();
    storage
        .connection
        .execute("UPDATE documents SET finished_at_least_once = 1", [])
        .unwrap();
    storage
        .save_document_reading_position(
            old.document.document_id,
            position(&old.document, bytes.len()),
        )
        .map(|value| value.map(crate::tests::as_paste))
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM documents WHERE id = ?1",
            [i64::from(old.document.document_id)],
        )
        .unwrap();
    storage
        .connection
        .execute(
            "DELETE FROM chapter_texts WHERE chapter_id = ?1",
            [i64::from(old.document.chapter.chapter_id)],
        )
        .unwrap();
    assert_eq!(count(&storage, "imported_files"), 0);
    assert!(srt_file(&storage.connection, old.document.document_id)
        .unwrap()
        .is_none());
    let new = created(
        storage
            .import_analyzed_srt(bytes, "new.srt", Some(TextEncoding::Utf8), 3, fixture)
            .unwrap(),
    );
    assert_ne!(new.document.document_id, old.document.document_id);
    assert_ne!(
        new.document.chapter.source_id,
        old.document.chapter.source_id
    );
    assert_eq!(new.registered_surfaces, 0);
    assert!(new.document.reading_position.is_none());
    assert!(!new.document.finished_at_least_once);
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct SrtFile {
    original_bytes: Vec<u8>,
    encoding: yomoku_import::txt::TextEncoding,
}
fn srt_file(
    connection: &rusqlite::Connection,
    id: yomoku_core::document::DocumentId,
) -> rusqlite::Result<Option<SrtFile>> {
    use rusqlite::OptionalExtension;
    connection.query_row("SELECT f.original_bytes, f.text_encoding FROM imported_files f JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id WHERE d.id = ?1 AND s.kind = 'srt'", [i64::from(id)], |row| {
        let encoding: String = row.get(1)?;
        Ok(SrtFile { original_bytes: row.get(0)?, encoding: match encoding.as_str() { "utf8" => yomoku_import::txt::TextEncoding::Utf8, "shift_jis" => yomoku_import::txt::TextEncoding::ShiftJis, _ => panic!("unexpected fixture encoding") } })
    }).optional()
}
