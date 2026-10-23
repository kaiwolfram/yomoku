//! SQLite I/O owned by the Rust runtime, never by the pure application core.
//!
//! Paste repositories use pure domain values. Runtime effect handling is separate.

mod documents;

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
        name: "0001_migration_history",
        sql: include_str!("../migrations/0001_migration_history.sql"),
    },
    Migration {
        version: 2,
        name: "0002_paste_documents",
        sql: include_str!("../migrations/0002_paste_documents.sql"),
    },
    Migration {
        version: 3,
        name: "0003_reading_positions",
        sql: include_str!("../migrations/0003_reading_positions.sql"),
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
        Ok(Self { connection })
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
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests;
