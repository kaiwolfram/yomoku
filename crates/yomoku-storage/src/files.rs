//! Exact original-byte identity shared by supported file adapters.

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use yomoku_core::document::{DocumentId, PastedDocument};
use yomoku_import::txt::TextEncoding;

use crate::{documents::load_paste_in, Storage, StorageError};

impl Storage {
    /// Read-only preflight across file kinds. Existing bytes retain the first
    /// committed interpretation even if selected with another name/extension.
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
    load_paste_in(connection, id)?.map_or(Err(StorageError::InvalidDocument), |document| {
        Ok(Some(document))
    })
}

pub(crate) fn encoding_name(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::ShiftJis => "shift_jis",
    }
}
