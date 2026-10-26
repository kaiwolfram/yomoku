//! Narrow verified asset reads. Never parse a container or load other BLOBs.

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use yomoku_core::{document::DocumentId, reader_images::ReaderImageAsset};
use yomoku_import::epub::EpubImage;

use crate::{Storage, StorageError};

/// Matches the import parser's default per-entry budget. Checked in SQL before
/// copying the selected BLOB into Rust, and before crossing JNI.
pub const MAX_EPUB_ASSET_BYTES: usize = 32 * 1024 * 1024;

impl Storage {
    /// Read exactly one document-scoped immutable image in a read transaction.
    /// Paths are opaque database keys, never filesystem paths or URLs. The
    /// expected media type comes from the validated reader/cover reference.
    /// Missing/deleted/non-EPUB documents and missing keys return None; damaged
    /// live EPUB metadata, changed media types or digests are errors, not repairs.
    pub fn load_epub_image_asset(
        &mut self,
        document_id: DocumentId,
        asset: &ReaderImageAsset,
    ) -> Result<Option<EpubImage>, StorageError> {
        asset
            .validate()
            .map_err(|_| StorageError::InvalidDocument)?;
        let transaction = self.connection.transaction()?;
        let header = transaction
            .query_row(
                "SELECT s.kind, e.document_id, f.text_encoding FROM documents d
             LEFT JOIN sources s ON s.id = d.source_id
             LEFT JOIN epub_documents e ON e.document_id = d.id
             LEFT JOIN imported_files f ON f.document_id = d.id WHERE d.id = ?1",
                [i64::from(document_id)],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((kind, metadata_id, encoding)) = header else {
            return Ok(None);
        };
        let kind = kind.ok_or(StorageError::InvalidDocument)?;
        if kind != "epub" {
            return Ok(None);
        }
        if metadata_id != Some(i64::from(document_id)) || encoding.as_deref() != Some("epub") {
            return Err(StorageError::InvalidDocument);
        }
        let saved = transaction
            .query_row(
                "SELECT media_type, sha256,
             CASE WHEN length(bytes) <= ?3 THEN bytes END
             FROM epub_assets WHERE document_id = ?1 AND path = ?2",
                (
                    i64::from(document_id),
                    &asset.path,
                    MAX_EPUB_ASSET_BYTES as i64,
                ),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((media_type, digest, bytes)) = saved else {
            return Ok(None);
        };
        let bytes = bytes.ok_or(StorageError::InvalidDocument)?;
        if media_type != asset.media_type || Sha256::digest(&bytes).as_slice() != digest {
            return Err(StorageError::InvalidDocument);
        }
        transaction.commit()?;
        Ok(Some(EpubImage { media_type, bytes }))
    }
}
