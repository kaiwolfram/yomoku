//! Exact original-byte identity shared by supported file adapters.

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use yomoku_core::document::{DocumentId, PastedDocument};
use yomoku_import::txt::TextEncoding;

use crate::{documents::load_paste_in, Storage, StorageError};

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

    /// Single-text preflight across TXT/SRT. Existing bytes retain the first
    /// interpretation. Multi-chapter adapters must use find_imported_file_id;
    /// this historical payload rejects EPUB rather than truncating its chapters.
    pub fn find_imported_file(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<PastedDocument>, StorageError> {
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
) -> Result<Option<PastedDocument>, StorageError> {
    let Some(id) = find_imported_id_in(connection, original_bytes, digest)? else {
        return Ok(None);
    };
    load_paste_in(connection, id)?.map_or(Err(StorageError::InvalidDocument), |document| {
        Ok(Some(document))
    })
}

/// Format-independent identity for multi-chapter adapters. Single-text callers
/// continue to load their complete historical payload through find_imported_in.
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
