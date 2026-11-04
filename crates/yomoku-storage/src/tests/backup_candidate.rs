use std::io::{self, Write};

use yomoku_backup::{decrypt_snapshot, encrypt_snapshot, Manifest, Password};

use super::*;
use crate::{BackupCandidate, CandidateError};

mod analyses;
mod controls;
mod documents;
mod formats;
mod vocabulary;

fn accept_unchanged(database: &TestDatabase, version: u32) {
    let before = fs::read(database.path()).unwrap();
    let expected = super::backup_snapshot::state(&Connection::open(database.path()).unwrap());
    let directory = database.0.join("accepted");
    let checked = candidate(&directory, &before)
        .check_schema(version, before.len() as u64, || false)
        .unwrap();
    let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
    assert_eq!(super::backup_snapshot::state(&copy), expected);
    drop(copy);
    checked.discard().unwrap();
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

fn candidate(directory: &Path, bytes: &[u8]) -> BackupCandidate {
    let mut candidate = BackupCandidate::create(directory).unwrap();
    // Streaming decryption may deliver arbitrary short writes.
    for chunk in bytes.chunks(127) {
        candidate.write_all(chunk).unwrap();
    }
    candidate.flush().unwrap();
    candidate
}

fn check_rejected(database: &TestDatabase, version: u32, expected: CandidateError) {
    let before = fs::read(database.path()).unwrap();
    let directory = database.0.join("candidate");
    let result =
        candidate(&directory, &before).check_schema(version, before.len() as u64, || false);
    assert!(matches!(result, Err(error) if error == expected));
    assert!(!directory.exists());
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn backup_candidate_encrypted_full_state_retains_every_table_and_journal() {
    let database = TestDatabase::new();
    let mut storage = Storage::open(database.path()).unwrap();
    super::backup_snapshot::populate(&mut storage);
    let before = super::backup_snapshot::state(&storage.connection);
    let mut snapshot = storage
        .create_backup_snapshot(database.0.join("snapshot"), || false)
        .unwrap();
    let mut encrypted = Vec::new();
    let manifest = Manifest::new(snapshot.schema_version(), snapshot.sqlite_len()).unwrap();
    encrypt_snapshot(
        Password::new(b"candidate-test".to_vec()),
        &manifest,
        &mut snapshot,
        &mut encrypted,
        || false,
    )
    .unwrap();
    let directory = database.0.join("candidate");
    let mut staging = BackupCandidate::create(&directory).unwrap();
    let manifest = decrypt_snapshot(
        Password::new(b"candidate-test".to_vec()),
        &mut encrypted.as_slice(),
        &mut staging,
        || false,
    )
    .unwrap();
    let checked = staging
        .check_schema(manifest.schema_version(), manifest.sqlite_len(), || false)
        .unwrap();
    assert_eq!(checked.original_schema(), 1);
    assert_eq!(checked.schema_version(), 1);
    // Test-only access; the production opaque result exposes no file or SQL.
    let copy = Connection::open(directory.join("snapshot.sqlite3")).unwrap();
    assert_eq!(super::backup_snapshot::state(&copy), before);
    drop(copy);
    assert_eq!(super::backup_snapshot::state(&storage.connection), before);
    checked.discard().unwrap();
    assert!(!directory.exists());
}

#[test]
fn backup_candidate_checks_initial_schema_only_in_private_copy() {
    {
        let version = 1;
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        drop(connection);
        let before = fs::read(database.path()).unwrap();
        let directory = database.0.join("candidate");
        let checked = candidate(&directory, &before)
            .check_schema(version as u32, before.len() as u64, || false)
            .unwrap_or_else(|error| panic!("version {version}: {error}"));
        assert_eq!(checked.original_schema(), version as u32);
        assert_eq!(checked.schema_version(), 1);
        let copy = Storage::open(directory.join("snapshot.sqlite3")).unwrap();
        assert_eq!(copy.schema_version().unwrap(), 1);
        assert_eq!(
            copy.page_length().unwrap(),
            yomoku_core::pagination::PageLength::default()
        );
        drop(copy);
        checked.discard().unwrap();
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn backup_candidate_rejects_unsupported_or_disagreeing_manifest_and_journal() {
    for (version, expected) in [
        (0, CandidateError::UnsupportedSchema),
        (25, CandidateError::UnsupportedSchema),
        (u32::MAX, CandidateError::UnsupportedSchema),
        (23, CandidateError::UnsupportedSchema),
    ] {
        let database = TestDatabase::new();
        let _storage = Storage::open(database.path()).unwrap();
        check_rejected(&database, version, expected);
    }
    for sql in [
        "DELETE FROM schema_migrations WHERE version = 1",
        "UPDATE schema_migrations SET name = 'untrusted secret' WHERE version = 1",
        "INSERT INTO schema_migrations VALUES (25, 'future')",
        "DELETE FROM schema_migrations",
    ] {
        let database = TestDatabase::new();
        let storage = Storage::open(database.path()).unwrap();
        storage.connection.execute_batch(sql).unwrap();
        check_rejected(&database, 1, CandidateError::InvalidDatabase);
    }
}

#[test]
fn backup_candidate_rejects_view_impersonating_migration_journal_before_querying_it() {
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch(
            "ALTER TABLE schema_migrations RENAME TO stolen;
         CREATE VIEW schema_migrations AS SELECT * FROM stolen;",
        )
        .unwrap();
    check_rejected(&database, 1, CandidateError::InvalidDatabase);
}

#[test]
fn backup_candidate_rejects_malformed_schema_without_leaking_sql() {
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    storage
        .connection
        .execute_batch(
            "PRAGMA writable_schema=ON;
         UPDATE sqlite_schema SET sql = 'SECRET invalid SQL' WHERE name = 'documents';",
        )
        .unwrap();
    drop(storage);
    check_rejected(&database, 1, CandidateError::Sqlite);
    assert_eq!(
        CandidateError::Sqlite.to_string(),
        "backup candidate SQLite operation failed"
    );
}

#[test]
fn backup_candidate_checks_constraints_and_foreign_keys_before_migration() {
    for (version, sql) in [
        (1, "PRAGMA ignore_check_constraints=ON; UPDATE reader_settings SET page_length=1"),
        (1, "PRAGMA ignore_check_constraints=ON; UPDATE reader_settings SET page_length=1"),
        (1, "PRAGMA foreign_keys=OFF; INSERT INTO documents (source_id, title) VALUES (99, 'orphan')"),
    ] {
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        connection.execute_batch(sql).unwrap();
        drop(connection);
        check_rejected(&database, version as u32, CandidateError::InvalidDatabase);
    }
}

#[test]
fn backup_candidate_rejects_bad_header_length_wal_and_trailing_bytes() {
    let database = TestDatabase::new();
    let _storage = Storage::open(database.path()).unwrap();
    let original = fs::read(database.path()).unwrap();
    for (index, bytes, declared) in [
        (0, vec![0; 512], 512),
        (1, original[..511].to_vec(), 511),
        (2, original.clone(), original.len() as u64 + 512),
        (
            3,
            {
                let mut b = original.clone();
                b[18] = 2;
                b[19] = 2;
                b
            },
            original.len() as u64,
        ),
        (
            4,
            {
                let mut b = original.clone();
                b.extend([0; 512]);
                b
            },
            original.len() as u64 + 512,
        ),
        (
            5,
            {
                let mut b = original.clone();
                b[16] = 0;
                b[17] = 3;
                b
            },
            original.len() as u64,
        ),
    ] {
        let directory = database.0.join(format!("bad-{index}"));
        assert!(matches!(
            candidate(&directory, &bytes).check_schema(1, declared, || false),
            Err(CandidateError::InvalidDatabase)
        ));
        assert!(!directory.exists());
    }
    assert_eq!(fs::read(database.path()).unwrap(), original);
}

#[test]
fn backup_candidate_cancellation_at_every_poll_cleans_current_copy() {
    {
        let version = 1;
        let database = TestDatabase::new();
        let mut connection = Connection::open(database.path()).unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        drop(connection);
        let bytes = fs::read(database.path()).unwrap();
        let directory = database.0.join("count");
        let mut polls = 0;
        candidate(&directory, &bytes)
            .check_schema(version as u32, bytes.len() as u64, || {
                polls += 1;
                false
            })
            .unwrap()
            .discard()
            .unwrap();
        for stop in 1..=polls {
            let directory = database.0.join(format!("cancel-{stop}"));
            let mut calls = 0;
            assert!(matches!(
                candidate(&directory, &bytes).check_schema(
                    version as u32,
                    bytes.len() as u64,
                    || {
                        calls += 1;
                        calls == stop
                    }
                ),
                Err(CandidateError::Cancelled)
            ));
            assert!(!directory.exists());
        }
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}

#[test]
fn backup_candidate_private_lifecycle_refuses_existing_paths_and_preserves_unowned_files() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let database = TestDatabase::new();
    let directory = database.0.join("private");
    let staging = BackupCandidate::create(&directory).unwrap();
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
    let link = database.0.join("symlink");
    symlink(&directory, &link).unwrap();
    for path in [&directory, &link] {
        assert!(matches!(
            BackupCandidate::create(path),
            Err(CandidateError::Io(io::ErrorKind::AlreadyExists))
        ));
    }
    for suffix in ["-journal", "-wal", "-shm"] {
        fs::write(
            directory.join(format!("snapshot.sqlite3{suffix}")),
            b"sidecar",
        )
        .unwrap();
    }
    staging.discard().unwrap();
    assert!(!directory.exists());
    let staging = BackupCandidate::create(&directory).unwrap();
    fs::write(directory.join("keep"), b"unowned").unwrap();
    assert!(matches!(staging.discard(), Err(CandidateError::Io(_))));
    assert_eq!(fs::read(directory.join("keep")).unwrap(), b"unowned");
    fs::remove_file(directory.join("keep")).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[test]
fn backup_candidate_failed_decryption_and_abandoned_write_discard_plaintext() {
    let database = TestDatabase::new();
    let directory = database.0.join("candidate");
    let staging = candidate(&directory, b"private partial plaintext");
    drop(staging);
    assert!(!directory.exists());
    let mut staging = BackupCandidate::create(&directory).unwrap();
    assert!(decrypt_snapshot(
        Password::new(b"secret".to_vec()),
        &mut b"bad archive".as_slice(),
        &mut staging,
        || false,
    )
    .is_err());
    drop(staging);
    assert!(!directory.exists());
}

#[test]
fn backup_candidate_io_failures_before_sqlite_open_clean_owned_directory() {
    let database = TestDatabase::new();
    let _storage = Storage::open(database.path()).unwrap();
    let before = fs::read(database.path()).unwrap();
    for stop in [1, 2] {
        let directory = database.0.join(format!("io-{stop}"));
        let staging = candidate(&directory, &before);
        let mut calls = 0;
        let result = staging.check_schema(1, before.len() as u64, || {
            calls += 1;
            if calls == stop {
                fs::remove_file(directory.join("snapshot.sqlite3")).unwrap();
            }
            false
        });
        assert!(matches!(
            result,
            Err(CandidateError::Io(io::ErrorKind::NotFound) | CandidateError::Sqlite)
        ));
        assert!(!directory.exists());
    }
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn backup_candidate_checks_compact_reservation_provenance_occurrences_and_shape() {
    use yomoku_core::{
        completion::PageCompletionIntent,
        document::PasteText,
        reader::{ReaderAnalysis, ReaderPage},
    };
    for change in [
        "json_remove(payload, '$.occurrences[0]')",
        "json_set(payload, '$.mode', 'B')",
        "json_set(payload, '$.occurrences[0].surface', '別')",
        "json_set(payload, '$.profile.engine_revision', 'different')",
        "json_set(payload, '$.key', json('{}'))",
    ] {
        let database = TestDatabase::new();
        let mut storage = Storage::open(database.path()).unwrap();
        let imported = storage
            .create_analyzed_paste(
                &PasteText::new("猫 犬".into()).unwrap(),
                0,
                imports::fixture,
            )
            .unwrap();
        let snapshot = storage
            .load_document_reader_snapshot(imported.document.document_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            snapshot.analyses[0].analysis,
            ReaderAnalysis::Available(imported.analysis_id)
        );
        let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
        let key = storage
            .load_effective_reader_tokens(imported.document.document_id, page.anchor)
            .unwrap()
            .unwrap()
            .key()
            .unwrap();
        let frozen = storage
            .prepare_effective_page_completion(
                PageCompletionIntent {
                    document_id: imported.document.document_id,
                    page: page.anchor,
                    page_length: snapshot.page_length,
                    departing_position: page.position,
                    completed_at_unix_ms: 1,
                    analysis_id: imported.analysis_id,
                },
                &key,
            )
            .unwrap();
        storage.record_effective_page_completion(&frozen).unwrap();
        accept_unchanged(&database, 1);
        storage
            .connection
            .execute(
                &format!("UPDATE effective_completion_commands SET payload = {change}"),
                [],
            )
            .unwrap();
        check_rejected(&database, 1, CandidateError::InvalidData);
    }
}
