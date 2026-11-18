//! SQLite I/O owned by the Rust runtime, never by the pure application core.
//!
//! Paste repositories use pure domain values. Runtime effect handling is separate.

mod analysis;
mod anki_settings;
mod completions;
pub mod dictionary;
mod documents;
mod effective_status;
mod encounters;
mod epub;
mod epub_assets;
mod files;
mod import_cancellation;
mod imports;
mod library_words;
mod reader_documents;
mod reader_integrity;
mod reader_tokens;
pub mod reader_words;
mod settings;
mod srt;
mod statistics;
mod txt;
mod vocabulary;
mod vocabulary_browser;

pub use epub::{
    AnalyzedEpub, EpubImportError, EpubImportOutcome, StoredEpubChapter, StoredEpubDocument,
};
pub use epub_assets::{EpubCoverReference, MAX_EPUB_ASSET_BYTES};
pub use files::{FilePreflight, PreparedFile};
pub use import_cancellation::ImportCancellation;
pub use imports::{AnalyzedPaste, PasteImportError, PasteImportOutcome};
pub use srt::{SrtImportError, SrtImportOutcome};
pub use txt::{TxtImportError, TxtImportOutcome};

use std::{error::Error, fmt, path::Path, time::Duration};

use rusqlite::{Connection, TransactionBehavior};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_completion_receipts_initial_schema",
        sql: include_str!("../migrations/0001_completion_receipts_initial_schema.sql"),
    },
    Migration {
        version: 2,
        name: "0002_library_read_override",
        sql: include_str!("../migrations/0002_library_read_override.sql"),
    },
    Migration {
        version: 3,
        name: "0003_hot_path_indexes",
        sql: include_str!("../migrations/0003_hot_path_indexes.sql"),
    },
];

/// Failures are returned to the runtime; opening never resets a database.
#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    UnsupportedSchemaVersion { found: i64, supported: i64 },
    InvalidMigrationHistory,
    UnrecognizedDatabase,
    InvalidDocument,
    InvalidSettings,
    InvalidCompletion,
    InvalidVocabulary,
    InvalidStatistics,
    InvalidAnalysis,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(_) => f.write_str("SQLite storage operation failed"),
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "database schema version {found} exceeds supported version {supported}"
            ),
            Self::InvalidMigrationHistory => {
                f.write_str("database migration history is inconsistent")
            }
            Self::UnrecognizedDatabase => {
                f.write_str("database has no recognized migration history")
            }
            Self::InvalidDocument => f.write_str("stored document is inconsistent"),
            Self::InvalidSettings => f.write_str("stored settings are inconsistent"),
            Self::InvalidCompletion => f.write_str("invalid or conflicting page completion"),
            Self::InvalidVocabulary => f.write_str("invalid vocabulary value or stored entry"),
            Self::InvalidStatistics => f.write_str("invalid statistics history or calendar date"),
            Self::InvalidAnalysis => f.write_str("invalid analysis or segmentation override"),
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

/// One owned connection. Call from the Rust I/O layer, away from the UI thread.
/// No raw SQL/connection access is exposed through this API or the FFI.
pub struct Storage {
    connection: Connection,
    validated_reader_selection: Option<reader_words::ValidatedSelection>,
    reader_integrity: reader_integrity::ReaderIntegrity,
}

impl Storage {
    /// Open or create a database in an existing directory and migrate atomically.
    /// The caller chooses the path (Android will supply its private files path).
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        // SQLite requires this outside a transaction, on every connection.
        connection.pragma_update(None, "foreign_keys", true)?;
        migrate(&mut connection, MIGRATIONS)?;
        let reader_integrity = reader_integrity::ReaderIntegrity::install(&mut connection)?;
        Ok(Self {
            reader_integrity,
            connection,
            validated_reader_selection: None,
        })
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        Ok(self
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })?)
    }
}

fn migrate(connection: &mut Connection, migrations: &[Migration]) -> Result<(), StorageError> {
    // Lock before reading history so concurrent opens cannot apply a migration twice.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let has_history: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |row| row.get(0),
    )?;
    let history = if has_history {
        let mut statement =
            transaction.prepare("SELECT version, name FROM schema_migrations ORDER BY version")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let history = rows.collect::<Result<Vec<_>, _>>()?;
        if history.is_empty() {
            return Err(StorageError::InvalidMigrationHistory);
        }
        history
    } else {
        let has_objects: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*')",
            [],
            |row| row.get(0),
        )?;
        if has_objects {
            return Err(StorageError::UnrecognizedDatabase);
        }
        Vec::new()
    };

    let supported = migrations.last().map_or(0, |migration| migration.version);
    if let Some((found, _)) = history.last() {
        if *found > supported {
            return Err(StorageError::UnsupportedSchemaVersion {
                found: *found,
                supported,
            });
        }
    }
    for ((version, name), migration) in history.iter().zip(migrations) {
        if *version != migration.version || name != migration.name {
            return Err(StorageError::InvalidMigrationHistory);
        }
    }
    if history.len() > migrations.len() {
        return Err(StorageError::InvalidMigrationHistory);
    }

    for migration in &migrations[history.len()..] {
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            (migration.version, migration.name),
        )?;
    }
    if history.len() < migrations.len() {
        let mut check = transaction.prepare("PRAGMA foreign_key_check")?;
        if check.query([])?.next()?.is_some() {
            return Err(StorageError::InvalidMigrationHistory);
        }
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests;
