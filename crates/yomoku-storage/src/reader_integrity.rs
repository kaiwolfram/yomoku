//! Connection-local integrity reuse. No payloads or analyses are cached here.

use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use yomoku_core::document::{DocumentId, SourceId};

use crate::{reader_words::SelectionRevision, StorageError};

#[derive(Clone, Copy, PartialEq, Eq)]
struct BinaryRevision {
    local: u64,
    external: i64,
    schema: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReaderRevision {
    database: SelectionRevision,
    binary: BinaryRevision,
}

#[derive(Default)]
pub(crate) struct ReaderIntegrity {
    binary_epoch: Arc<AtomicU64>,
    binaries: RefCell<Option<(DocumentId, BinaryRevision)>>,
    document: RefCell<Option<(DocumentId, ReaderRevision, SourceId)>>,
}

impl ReaderIntegrity {
    pub fn install(connection: &mut Connection) -> Result<Self, StorageError> {
        let integrity = Self::default();
        let epoch = integrity.binary_epoch.clone();
        // epub_assets is a rowid table. Trigger/cascade writes also fire the
        // hook. Metadata/count checks still detect deleted rows on every load.
        connection.update_hook(Some(move |_, database: &str, table: &str, _| {
            if database == "main" && table == "epub_assets" {
                epoch.fetch_add(1, Ordering::Relaxed);
            }
        }))?;
        let epoch = integrity.binary_epoch.clone();
        // A validation made against uncommitted bytes must not survive rollback.
        connection.rollback_hook(Some(move || {
            epoch.fetch_add(1, Ordering::Relaxed);
        }))?;
        Ok(integrity)
    }

    /// Read at transaction entry, before content establishes a SQLite snapshot.
    /// Retain this revision rather than sampling again after COMMIT.
    pub fn revision(&self, connection: &Connection) -> Result<ReaderRevision, StorageError> {
        let database = SelectionRevision::read(connection)?;
        Ok(ReaderRevision {
            binary: BinaryRevision {
                local: self.binary_epoch.load(Ordering::Relaxed),
                external: database.data_version,
                schema: database.schema_version,
            },
            database,
        })
    }

    pub fn validate_binaries(
        &self,
        connection: &Connection,
        document_id: DocumentId,
        revision: ReaderRevision,
    ) -> Result<(), StorageError> {
        let revision = revision.binary;
        if *self.binaries.borrow() == Some((document_id, revision)) {
            return Ok(());
        }
        self.binaries.replace(None);
        let mut statement =
            connection.prepare("SELECT bytes, sha256 FROM epub_assets WHERE document_id = ?1")?;
        let mut rows = statement.query([i64::from(document_id)])?;
        while let Some(row) = rows.next()? {
            if !valid_binary(row)? {
                return Err(StorageError::InvalidDocument);
            }
        }
        self.binaries.replace(Some((document_id, revision)));
        Ok(())
    }

    pub fn validated_source(
        &self,
        document_id: DocumentId,
        revision: ReaderRevision,
    ) -> Option<SourceId> {
        self.document
            .borrow()
            .filter(|(id, saved, _)| *id == document_id && *saved == revision)
            .map(|(_, _, source)| source)
    }

    pub fn remember_document(
        &self,
        document_id: DocumentId,
        source_id: SourceId,
        revision: ReaderRevision,
    ) {
        self.document
            .replace(Some((document_id, revision, source_id)));
    }
}

fn valid_binary(row: &rusqlite::Row<'_>) -> rusqlite::Result<bool> {
    // Borrow SQLite's current row rather than allocating another image.
    let bytes = row.get_ref(0)?.as_blob()?;
    let digest = row.get_ref(1)?.as_blob()?;
    Ok(Sha256::digest(bytes).as_slice() == digest)
}
