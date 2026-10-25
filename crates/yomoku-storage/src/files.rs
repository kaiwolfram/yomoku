//! Exact original-byte identity shared by supported file adapters.

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use yomoku_core::document::{DocumentId, LibraryDocument};
use yomoku_import::txt::TextEncoding;

use crate::{reader_documents::load_reader_document_in, Storage, StorageError};

impl Storage {
    /// Format-independent read-only preflight. A duplicate's identity is enough
    /// to reference its existing interpretation without a single-chapter payload.
    pub fn find_imported_file_id(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<DocumentId>, StorageError> {
        let digest = Sha256::digest(original_bytes);
        let transaction = self.connection.transaction()?;
        let id = find_imported_id_in(&transaction, original_bytes, digest.as_slice())?;
        transaction.commit()?;
        Ok(id)
    }

    /// Format-independent read-only reference across TXT/SRT/EPUB. Read the
    /// persisted title, whole-book progress and finish marker atomically with
    /// exact-byte identity, without decoding, analysis or resetting progress.
    pub fn find_imported_file(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<LibraryDocument>, StorageError> {
        let digest = Sha256::digest(original_bytes);
        let transaction = self.connection.transaction()?;
        let document = find_imported_in(&transaction, original_bytes, digest.as_slice())?;
        transaction.commit()?;
        Ok(document)
    }
}

pub(crate) fn find_imported_in(
    connection: &rusqlite::Connection,
    original_bytes: &[u8],
    digest: &[u8],
) -> Result<Option<LibraryDocument>, StorageError> {
    let Some(id) = find_imported_id_in(connection, original_bytes, digest)? else {
        return Ok(None);
    };
    let document = load_reader_document_in(connection, id)?.ok_or(StorageError::InvalidDocument)?;
    document
        .library_summary()
        .map(Some)
        .map_err(|_| StorageError::InvalidDocument)
}

/// Exact live file identity; callers may load a format-independent reference in
/// the same transaction through find_imported_in.
pub(crate) fn find_imported_id_in(
    connection: &rusqlite::Connection,
    original_bytes: &[u8],
    digest: &[u8],
) -> Result<Option<DocumentId>, StorageError> {
    let existing: Option<(i64, Vec<u8>)> = connection
        .query_row(
            "SELECT document_id, original_bytes FROM imported_files WHERE sha256 = ?1",
            [digest],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((id, stored_bytes)) = existing else {
        return Ok(None);
    };
    // Fail closed on a stale/corrupt key or digest collision.
    if stored_bytes != original_bytes {
        return Err(StorageError::InvalidDocument);
    }
    let id = DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?;
    Ok(Some(id))
}

pub(crate) fn encoding_name(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::ShiftJis => "shift_jis",
    }
}
