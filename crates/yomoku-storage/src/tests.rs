use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    },
};

use super::*;

mod analysis;
mod completions;
mod custom_dictionary;
mod document_deletion;
mod documents;
mod effective_completions;
mod effective_overrides;
mod effective_status;
mod encounters;
mod epub;
#[path = "../../yomoku-import/tests/support/epub.rs"]
mod epub_fixture;
mod finished;
mod import_dedup;
mod imports;
mod library_order;
mod library_words;
mod positions;
mod reader_documents;
mod reader_tokens;
mod reader_words;
mod reading_time;
mod registered_overrides;
mod segmentation;
mod settings;
mod srt;
mod txt;
mod vocabulary;

static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

// Historical fixtures cannot use current snapshot queries before migration 0011.
fn legacy_position(
    storage: &Storage,
    document: &yomoku_core::document::PastedDocument,
    position: yomoku_core::pagination::ReadingPosition,
) -> Option<yomoku_core::document::PastedDocument> {
    crate::documents::write_position_in(&storage.connection, document.document_id, position)
        .unwrap();
    Some(yomoku_core::document::PastedDocument {
        reading_position: Some(position),
        ..document.clone()
    })
}

pub(crate) struct TestDatabase(PathBuf);

impl TestDatabase {
    pub(crate) fn new() -> Self {
        // create_dir, rather than create_dir_all, refuses to reuse another test's files.
        let path = std::env::temp_dir().join(format!(
            "yomoku-storage-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_DATABASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.0.join("読書.sqlite3")
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn history(path: &Path) -> Vec<(i64, String)> {
    let connection = Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT version, name FROM schema_migrations ORDER BY version")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn failing_migration() -> Migration {
    Migration {
        version: 20,
        name: "0020_failure_fixture",
        sql: "CREATE TABLE partial_fixture (value TEXT); INSERT INTO partial_fixture VALUES ('途中'); INSERT INTO nonexistent VALUES (1);",
    }
}

#[test]
fn creates_empty_database_and_reopens_without_reapplying_migrations() {
    let database = TestDatabase::new();
    assert!(!database.path().exists());
    for _ in 0..3 {
        let storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 19);
    }
    assert_eq!(
        history(&database.path()),
        [
            (1, "0001_migration_history".into()),
            (2, "0002_paste_documents".into()),
            (3, "0003_reading_positions".into()),
            (4, "0004_reader_settings".into()),
            (5, "0005_page_completions".into()),
            (6, "0006_vocabulary".into()),
            (7, "0007_analysis_anchors".into()),
            (8, "0008_paste_content_hashes".into()),
            (9, "0009_pending_reading_time".into()),
            (10, "0010_completion_occurrences".into()),
            (11, "0011_finished_documents".into()),
            (12, "0012_txt_imports".into()),
            (13, "0013_srt_imports".into()),
            (14, "0014_epub_imports".into()),
            (15, "0015_reader_typography".into()),
            (16, "0016_reader_furigana".into()),
            (17, "0017_custom_dictionary".into()),
            (18, "0018_effective_completions".into()),
            (19, "0019_library_read_order".into())
        ]
    );
    let connection = Connection::open(database.path()).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn enables_foreign_keys_on_every_connection() {
    let database = TestDatabase::new();
    for _ in 0..2 {
        let storage = Storage::open(database.path()).unwrap();
        let enabled: bool = storage
            .connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert!(enabled);
        storage.connection.execute_batch("CREATE TEMP TABLE parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE child (parent_id INTEGER REFERENCES parent(id));").unwrap();
        assert!(storage
            .connection
            .execute("INSERT INTO child VALUES (99)", [])
            .is_err());
    }
}

#[test]
fn upgrades_existing_database_in_order_and_preserves_data_on_reopen() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
    connection.execute_batch("CREATE TABLE persistence_fixture (value TEXT NOT NULL); INSERT INTO persistence_fixture VALUES ('日本語𠮷📖');").unwrap();
    drop(connection);
    for _ in 0..2 {
        let storage = Storage::open(database.path()).unwrap();
        assert!(storage.connection.table_exists(None, "documents").unwrap());
        assert_eq!(
            storage
                .connection
                .query_row("SELECT value FROM persistence_fixture", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "日本語𠮷📖"
        );
        assert_eq!(history(&database.path()).len(), 19);
    }
}

#[test]
fn failed_upgrade_rolls_back_schema_data_and_history() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    let before = history(&database.path());
    let mut connection = Connection::open(database.path()).unwrap();
    connection.execute_batch("CREATE TABLE existing_fixture (value TEXT); INSERT INTO existing_fixture VALUES ('保存');").unwrap();
    assert!(matches!(
        migrate(
            &mut connection,
            &[
                Migration { ..MIGRATIONS[0] },
                Migration { ..MIGRATIONS[1] },
                Migration { ..MIGRATIONS[2] },
                Migration { ..MIGRATIONS[3] },
                Migration { ..MIGRATIONS[4] },
                Migration { ..MIGRATIONS[5] },
                Migration { ..MIGRATIONS[6] },
                Migration { ..MIGRATIONS[7] },
                Migration { ..MIGRATIONS[8] },
                Migration { ..MIGRATIONS[9] },
                Migration { ..MIGRATIONS[10] },
                Migration { ..MIGRATIONS[11] },
                Migration { ..MIGRATIONS[12] },
                Migration { ..MIGRATIONS[13] },
                Migration { ..MIGRATIONS[14] },
                Migration { ..MIGRATIONS[15] },
                Migration { ..MIGRATIONS[16] },
                Migration { ..MIGRATIONS[17] },
                Migration { ..MIGRATIONS[18] },
                failing_migration()
            ]
        ),
        Err(StorageError::Sqlite(_))
    ));
    drop(connection);
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(history(&database.path()), before);
    assert!(!storage
        .connection
        .table_exists(None, "partial_fixture")
        .unwrap());
    assert_eq!(
        storage
            .connection
            .query_row("SELECT value FROM existing_fixture", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "保存"
    );
}

#[test]
fn failed_initialization_rolls_back_all_pending_migrations_and_can_retry() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    assert!(matches!(
        migrate(
            &mut connection,
            &[
                Migration { ..MIGRATIONS[0] },
                Migration { ..MIGRATIONS[1] },
                Migration { ..MIGRATIONS[2] },
                Migration { ..MIGRATIONS[3] },
                Migration { ..MIGRATIONS[4] },
                Migration { ..MIGRATIONS[5] },
                Migration { ..MIGRATIONS[6] },
                Migration { ..MIGRATIONS[7] },
                Migration { ..MIGRATIONS[8] },
                Migration { ..MIGRATIONS[9] },
                Migration { ..MIGRATIONS[10] },
                Migration { ..MIGRATIONS[11] },
                Migration { ..MIGRATIONS[12] },
                Migration { ..MIGRATIONS[13] },
                Migration { ..MIGRATIONS[14] },
                Migration { ..MIGRATIONS[15] },
                Migration { ..MIGRATIONS[16] },
                Migration { ..MIGRATIONS[17] },
                Migration { ..MIGRATIONS[18] },
                failing_migration()
            ]
        ),
        Err(StorageError::Sqlite(_))
    ));
    assert!(!connection.table_exists(None, "schema_migrations").unwrap());
    assert!(!connection.table_exists(None, "partial_fixture").unwrap());
    assert!(!connection.table_exists(None, "documents").unwrap());
    drop(connection);
    assert_eq!(
        Storage::open(database.path())
            .unwrap()
            .schema_version()
            .unwrap(),
        19
    );
}

#[test]
fn rejects_newer_schema_without_rewriting_history() {
    let database = TestDatabase::new();
    drop(Storage::open(database.path()).unwrap());
    Connection::open(database.path())
        .unwrap()
        .execute(
            "INSERT INTO schema_migrations VALUES (20, '0020_future')",
            [],
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::UnsupportedSchemaVersion {
            found: 20,
            supported: 19
        })
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn rejects_changed_or_missing_migration_history_without_repairing_it() {
    for sql in [
        "UPDATE schema_migrations SET name = 'different' WHERE version = 1",
        "DELETE FROM schema_migrations",
        "UPDATE schema_migrations SET version = 0 WHERE version = 1", // simulate corruption
    ] {
        let database = TestDatabase::new();
        drop(Storage::open(database.path()).unwrap());
        let connection = Connection::open(database.path()).unwrap();
        connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        connection.execute(sql, []).unwrap();
        drop(connection);
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            Storage::open(database.path()),
            Err(StorageError::InvalidMigrationHistory)
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
    }
}

#[test]
fn rejects_gaps_in_history() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
    connection
        .execute(
            "INSERT INTO schema_migrations VALUES (3, '0003_fixture')",
            [],
        )
        .unwrap();
    let migrations = [
        Migration { ..MIGRATIONS[0] },
        Migration {
            version: 2,
            name: "0002_fixture",
            sql: "",
        },
        Migration {
            version: 3,
            name: "0003_fixture",
            sql: "",
        },
    ];
    assert!(matches!(
        migrate(&mut connection, &migrations),
        Err(StorageError::InvalidMigrationHistory)
    ));
    assert_eq!(
        history(&database.path())
            .iter()
            .map(|row| row.0)
            .collect::<Vec<_>>(),
        [1, 3]
    );
}

#[test]
fn rejects_unrecognized_database_without_changing_existing_data() {
    let database = TestDatabase::new();
    Connection::open(database.path())
        .unwrap()
        .execute_batch(
            "CREATE TABLE other_app (value TEXT); INSERT INTO other_app VALUES ('保持');",
        )
        .unwrap();
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::UnrecognizedDatabase)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
}

#[test]
fn returns_errors_for_missing_directory_and_non_database_files() {
    let database = TestDatabase::new();
    assert!(matches!(
        Storage::open(database.0.join("missing/db.sqlite3")),
        Err(StorageError::Sqlite(_))
    ));
    fs::write(database.path(), b"not a SQLite database").unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::Sqlite(_))
    ));
    assert_eq!(fs::read(database.path()).unwrap(), b"not a SQLite database");
}

#[test]
fn concurrent_initialization_records_migration_once() {
    let database = TestDatabase::new();
    let barrier = Arc::new(Barrier::new(4));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let barrier = Arc::clone(&barrier);
            let path = database.path();
            scope.spawn(move || {
                barrier.wait();
                assert_eq!(Storage::open(path).unwrap().schema_version().unwrap(), 19);
            });
        }
    });
    assert_eq!(history(&database.path()).len(), 19);
}
