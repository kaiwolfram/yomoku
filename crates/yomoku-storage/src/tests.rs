use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Barrier,
    },
};
use yomoku_core::document::{PasteText, PastedDocument};

use super::*;

mod analysis;
mod anki_settings;
mod completions;
mod document_deletion;
mod document_rename;
mod documents;
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
mod settings;
mod srt;
mod statistics;
mod txt;
mod vocabulary;
mod vocabulary_browser;
mod vocabulary_deletion;

// Thread-local observations of the real read paths, so status-cache tests can
// detect accidental whole-document/chapter work and unrelated vocabulary reads.
pub(crate) mod reader_work {
    use std::cell::{Cell, RefCell};

    thread_local! {
        pub static DOCUMENTS: Cell<usize> = const { Cell::new(0) };
        pub static CHAPTERS: Cell<usize> = const { Cell::new(0) };
        pub static SURFACES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    pub fn take() -> (usize, usize, Vec<String>) {
        (DOCUMENTS.replace(0), CHAPTERS.replace(0), SURFACES.take())
    }
}
static NEXT_DATABASE: AtomicU64 = AtomicU64::new(0);

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

#[test]
fn initial_schema_has_only_release_tables_and_reopens_without_writes() {
    let database = TestDatabase::new();
    let storage = Storage::open(database.path()).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 3);
    let tables: Vec<String> = storage.connection.prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT GLOB 'sqlite_*' ORDER BY name").unwrap().query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    let mut expected = vec![
        "sources",
        "chapters",
        "documents",
        "chapter_texts",
        "paste_content_hashes",
        "imported_files",
        "epub_documents",
        "epub_chapters",
        "epub_spans",
        "epub_assets",
        "epub_inline_images",
        "chapter_analyses",
        "analyzed_tokens",
        "vocabulary",
        "vocabulary_status_events",
        "reading_positions",
        "library_read_clock",
        "completion_commands",
        "reading_completions",
        "completion_analyses",
        "reading_occurrences",
        "reader_settings",
        "anki_settings",
        "statistics_settings",
        "schema_migrations",
    ];
    expected.sort();
    assert_eq!(tables, expected);
    let columns: Vec<String> = storage
        .connection
        .prepare("PRAGMA table_info(reading_completions)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!columns.iter().any(|name| name == "active_time_ms"));
    let command_columns: Vec<(String, String, bool)> = storage
        .connection
        .prepare("SELECT name, type, [notnull] FROM pragma_table_info('completion_commands')")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        command_columns,
        [
            ("id".into(), "INTEGER".into(), false),
            ("pending_fingerprint".into(), "BLOB".into(), false),
        ]
    );
    for invalid in ["zeroblob(31)", "zeroblob(33)", "printf('%032d', 0)"] {
        assert!(storage
            .connection
            .execute(
                &format!(
                    "INSERT INTO completion_commands (pending_fingerprint) VALUES ({invalid})"
                ),
                [],
            )
            .is_err());
    }
    let import_columns: Vec<String> = storage
        .connection
        .prepare("PRAGMA table_info(imported_files)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        import_columns,
        [
            "document_id",
            "sha256",
            "text_encoding",
            "canonical_text_sha256"
        ]
    );
    assert_eq!(storage.anki_default_deck().unwrap(), None);
    assert_eq!(
        storage.page_length().unwrap(),
        yomoku_core::pagination::PageLength::default()
    );
    assert_eq!(
        storage.reader_typography().unwrap(),
        yomoku_core::typography::ReaderTypography::default()
    );
    assert_eq!(
        history(&database.path()),
        [
            (1, "0001_completion_receipts_initial_schema".into()),
            (2, "0002_library_read_override".into()),
            (3, "0003_hot_path_indexes".into()),
        ]
    );
    assert_eq!(
        storage
            .connection
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert!(storage
        .connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
    drop(storage);
    let bytes = fs::read(database.path()).unwrap();
    for _ in 0..3 {
        let storage = Storage::open(database.path()).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 3);
        assert!(storage
            .connection
            .pragma_query_value(None, "foreign_keys", |r| r.get::<_, bool>(0))
            .unwrap());
        assert_eq!(fs::read(database.path()).unwrap(), bytes);
    }
}

#[test]
fn concurrent_initialization_records_initial_schema_once() {
    let database = TestDatabase::new();
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let barrier = barrier.clone();
            let path = database.path();
            scope.spawn(move || {
                barrier.wait();
                assert_eq!(Storage::open(path).unwrap().schema_version().unwrap(), 3);
            });
        }
    });
    assert_eq!(
        history(&database.path()),
        [
            (1, "0001_completion_receipts_initial_schema".into()),
            (2, "0002_library_read_override".into()),
            (3, "0003_hot_path_indexes".into()),
        ]
    );
}

#[test]
fn previous_version_one_journal_and_completion_payload_are_untouched() {
    let database = TestDatabase::new();
    let connection = Connection::open(database.path()).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL) STRICT;
             INSERT INTO schema_migrations VALUES (1, '0001_canonical_import_initial_schema');
             CREATE TABLE completion_commands (id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0)) STRICT;
             CREATE TABLE effective_completion_commands (
                 command_id INTEGER PRIMARY KEY REFERENCES completion_commands(id),
                 semantics_version INTEGER NOT NULL CHECK (semantics_version = 2),
                 payload TEXT NOT NULL CHECK (json_valid(payload))
             ) STRICT;
             INSERT INTO completion_commands DEFAULT VALUES;
             INSERT INTO effective_completion_commands VALUES (1, 2, '{\"occurrences\":[\"猫\"]}');",
        )
        .unwrap();
    drop(connection);
    let before = fs::read(database.path()).unwrap();
    assert!(matches!(
        Storage::open(database.path()),
        Err(StorageError::InvalidMigrationHistory)
    ));
    assert_eq!(fs::read(database.path()).unwrap(), before);
    assert_eq!(fs::read_dir(&database.0).unwrap().count(), 1);
}

#[test]
fn incompatible_development_unknown_and_future_databases_are_never_cleared() {
    for journal in [
        None,
        Some((1, "0001_initial_schema")),
        Some((1, "0001_simplified_initial_schema")),
        Some((1, "0001_base_text_initial_schema")),
        Some((1, "0001_migration_history")),
        Some((24, "0024_anki_settings")),
        Some((2, "0002_future")),
        Some((1, "unexpected")),
    ] {
        let database = TestDatabase::new();
        let connection = Connection::open(database.path()).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE retained (value TEXT); INSERT INTO retained VALUES ('読書𠮷');",
            )
            .unwrap();
        if let Some((version, name)) = journal {
            connection.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL);").unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations VALUES (?1, ?2)",
                    (version, name),
                )
                .unwrap();
        }
        drop(connection);
        let before = fs::read(database.path()).unwrap();
        assert!(matches!(
            Storage::open(database.path()),
            Err(StorageError::UnrecognizedDatabase
                | StorageError::InvalidMigrationHistory
                | StorageError::UnsupportedSchemaVersion { .. })
        ));
        assert_eq!(fs::read(database.path()).unwrap(), before);
        let connection = Connection::open(database.path()).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT value FROM retained", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "読書𠮷"
        );
    }
}

#[test]
fn migration_failure_rolls_back_schema_journal_and_data_without_disabling_foreign_keys() {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
    let steps = [Migration { version: 1, name: MIGRATIONS[0].name, sql: MIGRATIONS[0].sql }, Migration { version: 2, name: "0002_failure", sql: "CREATE TABLE partial_fixture (value TEXT); INSERT INTO partial_fixture VALUES ('途中'); INSERT INTO nonexistent VALUES (1);" }];
    assert!(matches!(
        migrate(&mut connection, &steps),
        Err(StorageError::Sqlite(_))
    ));
    assert!(!connection.table_exists(None, "partial_fixture").unwrap());
    assert_eq!(history(&database.path()).len(), 1);
    assert!(connection
        .pragma_query_value(None, "foreign_keys", |r| r.get::<_, bool>(0))
        .unwrap());
    let steps = [
        Migration {
            version: 1,
            name: MIGRATIONS[0].name,
            sql: MIGRATIONS[0].sql,
        },
        Migration {
            version: 2,
            name: "0002_future",
            sql: "CREATE TABLE future_fixture (id INTEGER PRIMARY KEY);",
        },
    ];
    migrate(&mut connection, &steps).unwrap();
    assert!(connection.table_exists(None, "future_fixture").unwrap());
    assert_eq!(history(&database.path()).len(), 2);
}

impl Storage {
    /// Seed deliberately missing analysis for storage integrity fixtures.
    fn unanalyzed_fixture(&mut self, text: &PasteText) -> Result<PastedDocument, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = crate::documents::create_paste_in(&transaction, text)?;
        transaction.commit()?;
        Ok(document)
    }
}

impl Storage {
    /// Seed independent imports, including identical text, without deduplication.
    fn analyzed_fixture<E>(
        &mut self,
        text: &PasteText,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(
            &yomoku_core::document::ChapterText,
        ) -> Result<yomoku_core::analysis::ChapterAnalysis, E>,
    ) -> Result<AnalyzedPaste, PasteImportError<E>> {
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let imported = crate::imports::analyze_document_in(
            &transaction,
            crate::documents::create_paste_in(&transaction, text)?,
            registered_at_unix_ms,
            analyze,
        )?;
        transaction.commit().map_err(StorageError::from)?;
        Ok(imported)
    }
}

fn as_paste(document: yomoku_core::document::ReaderDocument) -> PastedDocument {
    assert_eq!(document.chapters.len(), 1);
    PastedDocument {
        document_id: document.document_id,
        title: document.title,
        finished_at_least_once: document.finished_at_least_once,
        read_override: document.read_override,
        reading_position: document.reading_position,
        chapter: document.chapters.into_iter().next().unwrap(),
    }
}

impl Storage {
    fn prepare_fixture_completion(
        &mut self,
        intent: yomoku_core::completion::PageCompletionIntent,
    ) -> Result<yomoku_core::effective_reader::EffectivePageCompletion, StorageError> {
        let key = self
            .load_effective_reader_tokens(intent.document_id, intent.page)?
            .and_then(|s| s.key())
            .ok_or(StorageError::InvalidAnalysis)?;
        self.prepare_effective_page_completion(intent, &key)
    }
}

fn reserve_current(
    storage: &mut Storage,
    id: yomoku_core::document::DocumentId,
    date: i64,
) -> yomoku_core::effective_reader::EffectivePageCompletion {
    let snapshot = storage.load_document_reader_snapshot(id).unwrap().unwrap();
    let page =
        yomoku_core::reader::ReaderPage::open_document(&snapshot.document, snapshot.page_length)
            .unwrap();
    let yomoku_core::reader::ReaderAnalysis::Available(analysis_id) = snapshot
        .analyses
        .iter()
        .find(|a| a.chapter_id == page.anchor.chapter_id)
        .unwrap()
        .analysis
    else {
        panic!("fixture analysis")
    };
    storage
        .prepare_fixture_completion(yomoku_core::completion::PageCompletionIntent {
            document_id: id,
            page: page.anchor,
            departing_position: page.position,
            page_length: snapshot.page_length,
            analysis_id,
            completed_at_unix_ms: date,
        })
        .unwrap()
}
