//! Compact snapshots of the app-owned database. This is not an import validator.

use std::{
    error::Error,
    fmt, fs,
    fs::{File, OpenOptions},
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OpenFlags};

use crate::{Storage, MIGRATIONS};

/// Fixed diagnostics, without paths, SQL, user content or SQLite error messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    Io(io::ErrorKind),
    Sqlite,
    InvalidSnapshot,
    Cancelled,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io(_) => "backup snapshot I/O failed",
            Self::Sqlite => "backup snapshot SQLite operation failed",
            Self::InvalidSnapshot => "backup snapshot is inconsistent",
            Self::Cancelled => "backup snapshot cancelled",
        })
    }
}

impl Error for SnapshotError {}

impl From<io::Error> for SnapshotError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.kind())
    }
}

impl From<rusqlite::Error> for SnapshotError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Sqlite
    }
}

/// Read-only, structurally checked snapshot, independent of the live connection.
/// No paths, SQL access or authority to restore are exposed. Pass this reader and
/// its schema/length to the backup crate. Retrying encryption can rewind it.
///
/// Drop attempts to remove the plaintext file and its owned directory. Use
/// `discard` to observe cleanup errors. The caller must also clean abandoned
/// staging after process death and exclude its private parent from OS backup.
pub struct BackupSnapshot {
    file: Option<File>,
    staging: StagingDirectory,
    schema_version: u32,
    sqlite_len: u64,
}

impl BackupSnapshot {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn sqlite_len(&self) -> u64 {
        self.sqlite_len
    }

    pub fn rewind(&mut self) -> io::Result<()> {
        self.file.as_mut().expect("owned snapshot file").rewind()
    }

    pub fn discard(mut self) -> Result<(), SnapshotError> {
        drop(self.file.take());
        self.staging.cleanup()?;
        Ok(())
    }
}

impl Read for BackupSnapshot {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.as_mut().expect("owned snapshot file").read(bytes)
    }
}

impl Storage {
    /// Create a compact full-state copy using bound `VACUUM main INTO ?1`.
    ///
    /// `directory` must be a NEW child of a caller-owned app-private, non-backed-up
    /// parent. Existing directories/files/symlinks are refused, never reused.
    /// Linux/Android permissions are 0700 for the directory and 0600 for the file.
    /// Call on the storage FIFO after pending reader mutations drain while new mutations are gated.
    /// SQLite also gives a coherent snapshot with concurrent connections/WAL;
    /// mutations committed after that point are excluded. The source is untouched.
    ///
    /// Only the trusted application-origin copy is opened here, without CREATE or
    /// migrations, with query_only enabled. Integrity, foreign keys, the exact
    /// current migration journal and compact size are checked. This is NOT hardened validation of imported SQL
    /// or all application invariants. Never use it to authorize replacement.
    ///
    /// Cancellation is checked between synchronous calls; it cannot interrupt
    /// VACUUM, validation, blocking I/O or sync. Errors discard staging best effort;
    /// the caller must handle cleanup failures/crashes before app integration.
    pub fn create_backup_snapshot(
        &mut self,
        directory: impl AsRef<Path>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<BackupSnapshot, SnapshotError> {
        check_cancelled(&mut cancelled)?;
        let staging = StagingDirectory::create(directory.as_ref())?;
        check_cancelled(&mut cancelled)?;
        let path = staging.path();
        let path_text = path
            .to_str()
            .ok_or(SnapshotError::Io(io::ErrorKind::InvalidInput))?;
        self.connection
            .execute("VACUUM main INTO ?1", [path_text])?;
        check_cancelled(&mut cancelled)?;

        let sqlite_len = fs::metadata(&path)?.len();
        let schema_version = validate_app_snapshot(&path, sqlite_len)?;
        check_cancelled(&mut cancelled)?;
        // Do not depend on the source's synchronous policy for the copy's sync.
        let sync = OpenOptions::new().read(true).write(true).open(&path)?;
        sync.sync_all()?;
        drop(sync);
        check_cancelled(&mut cancelled)?;
        let file = File::open(&path)?;
        check_cancelled(&mut cancelled)?;
        Ok(BackupSnapshot {
            file: Some(file),
            staging,
            schema_version,
            sqlite_len,
        })
    }
}

fn check_cancelled(cancelled: &mut impl FnMut() -> bool) -> Result<(), SnapshotError> {
    if cancelled() {
        Err(SnapshotError::Cancelled)
    } else {
        Ok(())
    }
}

fn validate_app_snapshot(path: &Path, sqlite_len: u64) -> Result<u32, SnapshotError> {
    // SQLite omits parsed CHECK expressions for a read-only B-tree, so a purely
    // read-only connection's integrity_check misses invalid CHECK values. Open
    // only this owned private copy without CREATE and prohibit SQL writes before
    // schema parsing. The corrupt-settings regression guards CHECK validation.
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection.pragma_update(None, "query_only", true)?;
    let mut integrity = connection.prepare("PRAGMA main.integrity_check")?;
    let mut rows = integrity.query([])?;
    if rows
        .next()?
        .ok_or(SnapshotError::InvalidSnapshot)?
        .get::<_, String>(0)?
        != "ok"
        || rows.next()?.is_some()
    {
        return Err(SnapshotError::InvalidSnapshot);
    }
    if connection
        .prepare("PRAGMA main.foreign_key_check")?
        .query([])?
        .next()?
        .is_some()
    {
        return Err(SnapshotError::InvalidSnapshot);
    }
    let mut history =
        connection.prepare("SELECT version, name FROM main.schema_migrations ORDER BY version")?;
    let mut rows = history.query([])?;
    for migration in MIGRATIONS {
        let row = rows.next()?.ok_or(SnapshotError::InvalidSnapshot)?;
        if row.get::<_, i64>(0)? != migration.version || row.get::<_, String>(1)? != migration.name
        {
            return Err(SnapshotError::InvalidSnapshot);
        }
    }
    if rows.next()?.is_some() {
        return Err(SnapshotError::InvalidSnapshot);
    }
    let page_size: u32 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
    let pages: u32 = connection.pragma_query_value(None, "page_count", |row| row.get(0))?;
    let free: u32 = connection.pragma_query_value(None, "freelist_count", |row| row.get(0))?;
    if free != 0 || sqlite_len < 512 || u64::from(pages) * u64::from(page_size) != sqlite_len {
        return Err(SnapshotError::InvalidSnapshot);
    }
    u32::try_from(MIGRATIONS.last().expect("current schema").version)
        .map_err(|_| SnapshotError::InvalidSnapshot)
}

pub(super) struct StagingDirectory(PathBuf);

impl StagingDirectory {
    pub(super) fn create(path: &Path) -> Result<Self, SnapshotError> {
        #[cfg(not(unix))]
        return Err(SnapshotError::Io(io::ErrorKind::Unsupported));
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            fs::DirBuilder::new().mode(0o700).create(path)?;
            let mut staging = Self(path.to_owned());
            // Absolute paths cannot be interpreted as SQLite file: URIs. Bind
            // Unicode/quotes literally, without SQL interpolation.
            staging.0 = fs::canonicalize(path)?;
            drop(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(staging.path())?,
            );
            Ok(staging)
        }
    }

    pub(super) fn path(&self) -> PathBuf {
        self.0.join("snapshot.sqlite3")
    }

    pub(super) fn cleanup(&self) -> io::Result<()> {
        // Only our fixed database and SQLite sidecars, never recursive deletion.
        for name in [
            "snapshot.sqlite3",
            "snapshot.sqlite3-journal",
            "snapshot.sqlite3-wal",
            "snapshot.sqlite3-shm",
        ] {
            match fs::remove_file(self.0.join(name)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        match fs::remove_dir(&self.0) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
