//! Input file digest identity shared by supported file adapters.

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
        let id = find_imported_id_in(&transaction, digest.as_slice())?;
        transaction.commit()?;
        Ok(id)
    }

    /// Format-independent read-only reference across TXT/SRT/EPUB. Read the
    /// persisted title, whole-book progress and finish marker atomically with
    /// file digest identity, without decoding, analysis or resetting progress.
    pub fn find_imported_file(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<LibraryDocument>, StorageError> {
        let digest = Sha256::digest(original_bytes);
        let transaction = self.connection.transaction()?;
        let document = find_imported_in(&transaction, digest.as_slice())?;
        transaction.commit()?;
        Ok(document)
    }
}

pub(crate) fn find_imported_in(
    connection: &rusqlite::Connection,
    digest: &[u8],
) -> Result<Option<LibraryDocument>, StorageError> {
    let Some(id) = find_imported_id_in(connection, digest)? else {
        return Ok(None);
    };
    // Import duplicate checks always perform a fresh one-shot validation.
    let document = load_reader_document_in(
        connection,
        id,
        &crate::reader_integrity::ReaderIntegrity::default(),
    )?
    .ok_or(StorageError::InvalidDocument)?;
    let mut summary = document
        .library_summary()
        .map_err(|_| StorageError::InvalidDocument)?;
    summary.last_read_sequence = crate::documents::read_last_read_sequence_in(connection, id)?;
    Ok(Some(summary))
}

/// Live file identity; callers may load a format-independent reference in
/// the same transaction through find_imported_in.
pub(crate) fn find_imported_id_in(
    connection: &rusqlite::Connection,
    digest: &[u8],
) -> Result<Option<DocumentId>, StorageError> {
    let existing: Option<i64> = connection
        .query_row(
            "SELECT document_id FROM imported_files WHERE sha256 = ?1",
            [digest],
            |row| row.get(0),
        )
        .optional()?;
    existing
        .map(|id| DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument))
        .transpose()
}

pub(crate) fn store_imported_text_in(
    connection: &rusqlite::Connection,
    document_id: DocumentId,
    digest: &[u8],
    encoding: TextEncoding,
    text: &str,
) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO imported_files (document_id, sha256, text_encoding, canonical_text_sha256)
         VALUES (?1, ?2, ?3, ?4)",
        (
            i64::from(document_id),
            digest,
            encoding_name(encoding),
            Sha256::digest(text.as_bytes()).as_slice(),
        ),
    )?;
    Ok(())
}

fn encoding_name(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::ShiftJis => "shift_jis",
    }
}
