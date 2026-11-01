use std::{
    fs::File,
    io::{self, Read, Write},
};

use rusqlite::types::Value;
use yomoku_backup::{decrypt_snapshot, encrypt_snapshot, BackupError, Manifest, Password};
use yomoku_core::{
    active_time::PendingReadingTime,
    analysis::{OverrideScope, SegmentationOverride},
    anki::{AnkiModelCreation, AnkiModelLayout},
    completion::PageCompletionIntent,
    document::{PasteText, TextAnchor, TextOffset, TextRange},
    pagination::PageLength,
    reader::ReaderPage,
    typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography},
    vocabulary::{Surface, VocabularyStatus},
};
use yomoku_import::txt::TextEncoding;

use super::{imports::fixture, *};
use crate::{anki::AnkiSubmissionOutcome, anki_model::AnkiModelCreationOutcome};

// Snapshot every table and schema object, including sequences; no list of
// selected tables that could silently omit future durable state.
fn rows(connection: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut statement = connection.prepare(sql).unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| (0..columns).map(|index| row.get(index)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn state(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables = rows(
        connection,
        "SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name",
    );
    let mut state = vec![(
        "schema".into(),
        rows(
            connection,
            "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name",
        ),
    )];
    for table in tables {
        let Value::Text(name) = &table[0] else {
            panic!("table name")
        };
        let table = format!("\"{}\"", name.replace('"', "\"\""));
        let columns = connection
            .prepare(&format!("SELECT * FROM {table}"))
            .unwrap()
            .column_count();
        let order = (1..=columns)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        state.push((
            name.clone(),
            rows(
                connection,
                &format!("SELECT * FROM {table} ORDER BY {order}"),
            ),
        ));
    }
    state
}

fn password() -> Password {
    Password::new("  読書𠮷e\u{301}\n".as_bytes().to_vec())
}

fn complete(storage: &mut Storage, imported: &AnalyzedPaste) {
    let id = imported.document.document_id;
    let snapshot = storage.open_document_reader_snapshot(id).unwrap().unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    let saved = storage
        .load_pending_reading_time(id, page.position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(1234).unwrap())
        .unwrap();
    let key = storage
        .load_effective_reader_tokens(id, page.anchor)
        .unwrap()
        .unwrap()
        .key()
        .unwrap();
    let prepared = storage
        .prepare_effective_page_completion(
            PageCompletionIntent {
                document_id: id,
                page: page.anchor,
                page_length: snapshot.page_length,
                departing_position: page.position,
                completed_at_unix_ms: 500,
                analysis_id: imported.analysis_id,
            },
            &key,
        )
        .unwrap();
    storage.record_effective_page_completion(&prepared).unwrap();
}

const HIGH_WATER: i64 = 9_007_199_254_740_993;

fn populate(storage: &mut Storage) {
    let deleted = storage
        .create_analyzed_paste(&PasteText::new("猫 猫 犬 𠮷".into()).unwrap(), 100, fixture)
        .unwrap();
    let chapter = &deleted.document.chapter;
    for scope in [OverrideScope::AllOccurrences, OverrideScope::ThisOccurrence] {
        storage
            .save_segmentation_override(&SegmentationOverride {
                scope,
                anchor: TextAnchor {
                    source_id: chapter.source_id,
                    chapter_id: chapter.chapter_id,
                    range: TextRange {
                        start: TextOffset::START,
                        end: TextOffset::at(&chapter.text, "猫".len()).unwrap(),
                    },
                },
                surface: Surface::new("猫").unwrap(),
                parts: vec![Surface::new("猫").unwrap()],
            })
            .unwrap();
    }
    complete(storage, &deleted);
    storage
        .delete_document(deleted.document.document_id)
        .unwrap();
    let PasteImportOutcome::Created(live) = storage
        .import_analyzed_paste(
            &PasteText::new("残る 猫 が か\u{3099} English".into()).unwrap(),
            600,
            fixture,
        )
        .unwrap()
    else {
        panic!("new paste")
    };
    complete(storage, &live);
    // A backward reread retains the permanent finish marker and independent time.
    let position = yomoku_core::pagination::ReadingPosition {
        source_id: live.document.chapter.source_id,
        chapter_id: live.document.chapter.chapter_id,
        offset: TextOffset::START,
    };
    storage
        .save_reading_position(live.document.document_id, position)
        .unwrap();
    let saved = storage
        .load_pending_reading_time(live.document.document_id, position)
        .unwrap();
    storage
        .save_pending_reading_time(saved, PendingReadingTime::from_millis(9876).unwrap())
        .unwrap();
    // Leave an unfinished frozen reservation too; backups must not replay it.
    let page = ReaderPage::open_document(
        &storage
            .load_document_reader_snapshot(live.document.document_id)
            .unwrap()
            .unwrap()
            .document,
        storage.page_length().unwrap(),
    )
    .unwrap();
    storage
        .prepare_page_completion(PageCompletionIntent {
            document_id: live.document.document_id,
            page: page.anchor,
            page_length: storage.page_length().unwrap(),
            departing_position: position,
            completed_at_unix_ms: 700,
            analysis_id: live.analysis_id,
        })
        .unwrap();
    storage
        .import_analyzed_txt(
            "｜猫《ねこ》 犬".as_bytes(),
            "原本.txt",
            Some(TextEncoding::Utf8),
            800,
            fixture,
        )
        .unwrap();
    storage
        .import_analyzed_srt(
            "1\n00:00:01,000 --> 00:00:02,000\n鳥\n".as_bytes(),
            "原本.srt",
            Some(TextEncoding::Utf8),
            800,
            fixture,
        )
        .unwrap();
    storage.import_analyzed_epub(&epub_fixture::book(true, "<p><ruby>猫<rt>ねこ</rt></ruby> <em>犬</em><img src='../images/cover.png' alt='絵'/></p>"), "原本.epub", 800, fixture).unwrap();
    for (word, status) in [
        ("新語", VocabularyStatus::New),
        ("学習", VocabularyStatus::Learning),
        ("慣れ", VocabularyStatus::Familiar),
        ("既知", VocabularyStatus::Known),
        ("除外", VocabularyStatus::Ignored),
    ] {
        let surface = Surface::new(word).unwrap();
        storage
            .register_vocabulary(std::slice::from_ref(&surface), 900)
            .unwrap();
        let entry = storage.vocabulary_entry(&surface).unwrap().unwrap();
        storage
            .set_vocabulary_status(entry.id, status, 1000)
            .unwrap();
    }
    storage.connection.execute("INSERT INTO custom_dictionary_entries VALUES ('𠮷', 1, 'literal <meaning>\nsecond line', 'よし')", []).unwrap();
    storage.save_page_length(PageLength::Characters750).unwrap();
    storage
        .save_reader_typography(ReaderTypography {
            font_size: ReaderFontSize::Sp24,
            line_spacing: ReaderLineSpacing::Percent200,
            furigana: true,
        })
        .unwrap();
    storage
        .save_anki_default_deck(Some(HIGH_WATER.try_into().unwrap()))
        .unwrap();
    for outcome in [
        AnkiSubmissionOutcome::NotInserted,
        AnkiSubmissionOutcome::Added(i64::MAX.try_into().unwrap()),
    ] {
        let live = storage
            .begin_anki_submission(&anki::payload(HIGH_WATER))
            .unwrap();
        storage.finish_anki_submission(&live, outcome).unwrap();
    }
    for outcome in [
        AnkiModelCreationOutcome::NotCreated,
        AnkiModelCreationOutcome::Created(i64::MAX.try_into().unwrap()),
    ] {
        let live = storage
            .begin_anki_model_creation(&AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap())
            .unwrap();
        storage.finish_anki_model_creation(&live, outcome).unwrap();
    }
    storage
        .connection
        .execute("UPDATE sqlite_sequence SET seq = ?1", [HIGH_WATER])
        .unwrap();
}

#[test]
fn full_state_encrypted_round_trip_preserves_every_table_and_remains_usable() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    populate(&mut storage);
    let expected = state(&storage.connection);
    for (name, rows) in &expected {
        assert!(!rows.is_empty(), "uncovered table: {name}");
    }
    let totals = storage.lifetime_reading_totals().unwrap();
    let timeline = storage.vocabulary_status_timeline().unwrap();
    let library = storage.list_documents().unwrap();
    let before = fs::read(database.path()).unwrap();
    let directory = database.0.join("snapshot ' 日本語");
    let mut snapshot = storage
        .create_backup_snapshot(&directory, || false)
        .unwrap();
    let manifest = Manifest::new(snapshot.schema_version(), snapshot.sqlite_len()).unwrap();
    let mut archive = Vec::new();
    encrypt_snapshot(password(), &manifest, &mut snapshot, &mut archive, || false).unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), before);
    // Later source changes cannot affect the frozen reader.
    storage.save_page_length(PageLength::Characters250).unwrap();
    snapshot.rewind().unwrap();
    let mut frozen = Vec::new();
    snapshot.read_to_end(&mut frozen).unwrap();
    let candidate = TestDatabase::new();
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(candidate.path())
        .unwrap();
    assert_eq!(
        decrypt_snapshot(password(), &mut archive.as_slice(), &mut output, || false).unwrap(),
        manifest
    );
    output.sync_all().unwrap();
    drop(output);
    assert_eq!(fs::read(candidate.path()).unwrap(), frozen);
    snapshot.discard().unwrap();
    assert!(!directory.exists());
    // This is our own known fixture, not an untrusted import validation path.
    let mut restored = Storage::open(candidate.path()).unwrap();
    assert_eq!(state(&restored.connection), expected);
    assert_eq!(restored.list_documents().unwrap(), library);
    assert_eq!(restored.lifetime_reading_totals().unwrap(), totals);
    assert_eq!(restored.vocabulary_status_timeline().unwrap(), timeline);
    let imported = restored
        .create_analyzed_paste(
            &PasteText::new("復元後の新規語".into()).unwrap(),
            2000,
            fixture,
        )
        .unwrap();
    assert_eq!(i64::from(imported.document.document_id), HIGH_WATER + 1);
    assert_eq!(
        i64::from(imported.document.chapter.source_id),
        HIGH_WATER + 1
    );
    assert_eq!(
        i64::from(imported.document.chapter.chapter_id),
        HIGH_WATER + 1
    );
    assert_eq!(i64::from(imported.analysis_id), HIGH_WATER + 1);
    assert_eq!(
        i64::from(restored.new_completion_command_id().unwrap()),
        HIGH_WATER + 1
    );
    assert_ne!(fs::read(database.path()).unwrap(), before); // explicit source setting write
    assert_eq!(storage.page_length().unwrap(), PageLength::Characters250);
}

#[test]
fn snapshots_preserve_pending_and_uncertain_anki_journals_without_replay() {
    for model in [false, true] {
        for uncertain in [false, true] {
            let database = TestDatabase::new();
            let mut storage = Storage::open(database.path()).unwrap();
            if model {
                let live = storage
                    .begin_anki_model_creation(
                        &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                    )
                    .unwrap();
                if uncertain {
                    storage
                        .finish_anki_model_creation(
                            &live,
                            AnkiModelCreationOutcome::Uncertain {
                                known_model_id: Some(i64::MAX.try_into().unwrap()),
                            },
                        )
                        .unwrap();
                }
            } else {
                let live = storage
                    .begin_anki_submission(&anki::payload(HIGH_WATER))
                    .unwrap();
                if uncertain {
                    storage
                        .finish_anki_submission(
                            &live,
                            AnkiSubmissionOutcome::Uncertain {
                                known_note_id: Some(i64::MAX.try_into().unwrap()),
                            },
                        )
                        .unwrap();
                }
            }
            let recovery = storage.anki_recovery_snapshot().unwrap();
            let expected = state(&storage.connection);
            let before = fs::read(database.path()).unwrap();
            let mut snapshot = storage
                .create_backup_snapshot(database.0.join("snapshot"), || false)
                .unwrap();
            let manifest = Manifest::new(snapshot.schema_version(), snapshot.sqlite_len()).unwrap();
            let mut archive = Vec::new();
            encrypt_snapshot(password(), &manifest, &mut snapshot, &mut archive, || false).unwrap();
            let candidate = TestDatabase::new();
            let mut output = File::create(candidate.path()).unwrap();
            decrypt_snapshot(password(), &mut archive.as_slice(), &mut output, || false).unwrap();
            drop(output);
            let mut restored = Storage::open(candidate.path()).unwrap();
            assert_eq!(restored.anki_recovery_snapshot().unwrap(), recovery);
            assert_eq!(state(&restored.connection), expected);
            assert!(restored.begin_anki_submission(&anki::payload(1)).is_err());
            assert!(restored
                .begin_anki_model_creation(
                    &AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap()
                )
                .is_err());
            assert_eq!(state(&restored.connection), expected);
            assert_eq!(fs::read(database.path()).unwrap(), before);
        }
    }
}

#[test]
fn compact_snapshot_removes_deleted_free_page_content_and_leaves_source_untouched() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch("PRAGMA secure_delete = OFF")
        .unwrap();
    let marker = "deleted-body-marker-927c929a";
    let document = storage
        .create_paste(&PasteText::new(format!("削除本文\n{}", marker.repeat(10000))).unwrap())
        .unwrap();
    storage.delete_document(document.document_id).unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(before
        .windows(marker.len())
        .any(|bytes| bytes == marker.as_bytes()));
    let mut snapshot = storage
        .create_backup_snapshot(database.0.join("snapshot"), || false)
        .unwrap();
    let mut compact = Vec::new();
    snapshot.read_to_end(&mut compact).unwrap();
    assert!(compact.len() < before.len());
    assert!(!compact
        .windows(marker.len())
        .any(|bytes| bytes == marker.as_bytes()));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    let directory = database.0.join("snapshot");
    drop(snapshot);
    assert!(!directory.exists());
}

#[test]
fn wal_snapshot_includes_committed_wal_and_excludes_uncommitted_work() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch("PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0")
        .unwrap();
    let committed = storage
        .create_paste(&PasteText::new("WALの本文𠮷".into()).unwrap())
        .unwrap();
    let writer = Connection::open(database.path()).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE; UPDATE documents SET title = 'uncommitted'; UPDATE reader_settings SET page_length = 250").unwrap();
    let main = fs::read(database.path()).unwrap();
    let wal_path = database.path().with_extension("sqlite3-wal");
    let wal = fs::read(&wal_path).unwrap();
    let expected = state(&storage.connection);
    let snapshot = storage
        .create_backup_snapshot(database.0.join("snapshot"), || false)
        .unwrap();
    let copy = Connection::open_with_flags(
        database.0.join("snapshot/snapshot.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(state(&copy), expected);
    assert_eq!(fs::read(database.path()).unwrap(), main);
    assert_eq!(fs::read(wal_path).unwrap(), wal);
    writer.execute_batch("COMMIT").unwrap();
    assert_eq!(
        rows(&copy, "SELECT title FROM documents"),
        vec![vec![Value::Text(committed.title)]]
    );
    drop(copy);
    drop(snapshot);
}

#[test]
fn snapshot_refuses_existing_files_directories_and_symlinks_and_uses_private_modes() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let existing = database.0.join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("keep"), "preserved").unwrap();
    let link = database.0.join("link");
    symlink(&existing, &link).unwrap();
    for path in [&existing, &link, &database.path()] {
        assert!(matches!(
            storage.create_backup_snapshot(path, || false),
            Err(SnapshotError::Io(io::ErrorKind::AlreadyExists))
        ));
    }
    assert_eq!(fs::read(existing.join("keep")).unwrap(), b"preserved");
    assert!(link.is_symlink());
    let directory = database.0.join("private");
    let snapshot = storage
        .create_backup_snapshot(&directory, || false)
        .unwrap();
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(directory.join("snapshot.sqlite3"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    snapshot.discard().unwrap();
    assert!(!directory.exists());
}

#[test]
fn every_snapshot_cancellation_boundary_cleans_staging_and_preserves_source() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    let mut count = 0;
    storage
        .create_backup_snapshot(database.0.join("count"), || {
            count += 1;
            false
        })
        .unwrap()
        .discard()
        .unwrap();
    assert_eq!(count, 6);
    for stop in 1..=count {
        let directory = database.0.join(format!("cancel-{stop}"));
        let mut calls = 0;
        assert!(matches!(
            storage.create_backup_snapshot(&directory, || {
                calls += 1;
                calls == stop
            }),
            Err(SnapshotError::Cancelled)
        ));
        assert!(!directory.exists());
        assert_eq!(fs::read(database.path()).unwrap(), before);
        assert!(storage.connection.is_autocommit());
    }
}

#[test]
fn staging_io_failures_clean_owned_files_and_explicit_discard_reports_cleanup_errors() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    // Remove our disposable output just before validation, sync or reader open.
    // The source is never an output path even when later filesystem work fails.
    for stop in [3, 4, 5] {
        let directory = database.0.join(format!("fault-{stop}"));
        let mut count = 0;
        let error = match storage.create_backup_snapshot(&directory, || {
            count += 1;
            if count == stop {
                fs::remove_file(directory.join("snapshot.sqlite3")).unwrap();
            }
            false
        }) {
            Ok(_) => panic!("accepted missing staging"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SnapshotError::Sqlite | SnapshotError::Io(io::ErrorKind::NotFound)
        ));
        assert!(!directory.exists());
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    let directory = database.0.join("discard-failure");
    let snapshot = storage
        .create_backup_snapshot(&directory, || false)
        .unwrap();
    // Cleanup never recursively deletes unexpected files, and errors are visible.
    fs::write(directory.join("keep"), "unrelated").unwrap();
    assert_eq!(
        snapshot.discard(),
        Err(SnapshotError::Io(io::ErrorKind::DirectoryNotEmpty))
    );
    assert_eq!(fs::read(directory.join("keep")).unwrap(), b"unrelated");
    assert!(!directory.join("snapshot.sqlite3").exists());
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn validation_and_vacuum_failures_discard_only_owned_staging_and_allow_retry() {
    for corrupt in [
        "UPDATE schema_migrations SET name = 'wrong' WHERE version = 1",
        "DELETE FROM schema_migrations WHERE version = 24",
        "PRAGMA foreign_keys = OFF; INSERT INTO chapters VALUES (1, 123, 0)",
        "PRAGMA ignore_check_constraints = ON; UPDATE reader_settings SET page_length = -1",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(corrupt).unwrap();
        let before = fs::read(database.path()).unwrap();
        let directory = database.0.join("invalid");
        let error = match storage.create_backup_snapshot(&directory, || false) {
            Ok(_) => panic!("accepted corruption: {corrupt}"),
            Err(error) => error,
        };
        assert_eq!(error, SnapshotError::InvalidSnapshot, "{corrupt}");
        assert!(!directory.exists());
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    storage.connection.execute_batch("BEGIN").unwrap();
    let directory = database.0.join("transaction");
    assert!(matches!(
        storage.create_backup_snapshot(&directory, || false),
        Err(SnapshotError::Sqlite)
    ));
    assert!(!directory.exists());
    storage.connection.execute_batch("ROLLBACK").unwrap();
    storage
        .create_backup_snapshot(&directory, || false)
        .unwrap()
        .discard()
        .unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn bad_password_tampering_and_output_failure_leave_live_database_intact() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    storage
        .create_paste(&PasteText::new("保護される本文".repeat(30000)).unwrap())
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    let directory = database.0.join("snapshot");
    let mut snapshot = storage
        .create_backup_snapshot(&directory, || false)
        .unwrap();
    let manifest = Manifest::new(snapshot.schema_version(), snapshot.sqlite_len()).unwrap();
    struct FailedOutput;
    impl Write for FailedOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::StorageFull))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        encrypt_snapshot(
            password(),
            &manifest,
            &mut snapshot,
            &mut FailedOutput,
            || false
        ),
        Err(BackupError::Io(io::ErrorKind::StorageFull))
    );
    snapshot.rewind().unwrap();
    let mut archive = Vec::new();
    encrypt_snapshot(password(), &manifest, &mut snapshot, &mut archive, || false).unwrap();
    let last = archive.len() - 1;
    for tamper in [false, true] {
        let candidate = TestDatabase::new();
        let mut output = File::create(candidate.path()).unwrap();
        let attempt = if tamper {
            archive[last] ^= 1;
            password()
        } else {
            Password::new(b"wrong".to_vec())
        };
        assert_eq!(
            decrypt_snapshot(attempt, &mut archive.as_slice(), &mut output, || false),
            Err(BackupError::Authentication)
        );
        drop(output);
        // Dispose even though a late failed tag may have left plaintext chunks.
        fs::remove_file(candidate.path()).unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
    snapshot.discard().unwrap();
    assert!(!directory.exists());
}
