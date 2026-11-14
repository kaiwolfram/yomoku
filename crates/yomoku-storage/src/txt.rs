//! Atomic original-file identity, canonical TXT content.

use std::{error::Error, fmt};

use rusqlite::{OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, DocumentId, LibraryDocument},
};
use yomoku_import::txt::{self, TextEncoding, TxtError};

use crate::{
    documents::create_single_text_in,
    files::{encoding_name, find_imported_in},
    imports::store_analyzed_document_in,
    AnalyzedPaste, ImportCancellation, Storage, StorageError,
};

#[derive(Debug)]
pub enum TxtImportOutcome {
    Created(AnalyzedPaste),
    Existing(LibraryDocument),
}

#[derive(Debug)]
pub enum TxtImportError<E> {
    Parse(TxtError),
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for TxtImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Analysis(error) => write!(f, "TXT analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for TxtImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for TxtImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

/// Original bytes and the selected decoder for a saved TXT source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredTxtSource {
    pub original_bytes: Vec<u8>,
    pub encoding: TextEncoding,
}

impl Storage {
    /// Compatibility preflight: references any supported live file format.
    /// Exact duplicates need no decoder/analyzer.
    /// New files still require the authoritative lookup in the import transaction.
    pub fn find_imported_txt(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<LibraryDocument>, StorageError> {
        self.find_imported_file(original_bytes)
    }

    /// Hash original bytes before decoding; existing live files bypass parsing,
    /// analysis and registration, preserving their title, encoding and progress.
    /// For a new file, filename is its literal display name (not a URI/path).
    /// Run on a worker with a ready analyzer. No provisional IDs may escape the
    /// callback; it must not perform database work. Errors discard this file only.
    pub fn import_analyzed_txt<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<TxtImportOutcome, TxtImportError<E>> {
        self.import_analyzed_txt_cancellable(
            original_bytes,
            filename,
            selection,
            registered_at_unix_ms,
            &ImportCancellation::default(),
            analyze,
        )
        // This private handle is never shared or cancelled.
        .map(|outcome| outcome.expect("unshared import cannot be cancelled"))
    }

    /// Like `import_analyzed_txt`, with cooperative per-file cancellation.
    /// `Ok(None)` means cancelled: all partial rows and allocated IDs roll back.
    /// `Ok(Some(..))` means completed, even if cancellation arrives after COMMIT.
    /// Errors remain errors, with the same rollback guarantee. Share a fresh
    /// cancellation handle before submitting each file to a worker; never queue
    /// the cancellation request behind that worker's import. No partial IDs escape.
    pub fn import_analyzed_txt_cancellable<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        cancellation: &ImportCancellation,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<Option<TxtImportOutcome>, TxtImportError<E>> {
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let digest = Sha256::digest(original_bytes);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let outcome = if let Some(document) =
            find_imported_in(&transaction, original_bytes, digest.as_slice())?
        {
            TxtImportOutcome::Existing(document)
        } else {
            if filename.trim().is_empty() {
                return Err(StorageError::InvalidDocument.into());
            }
            let parsed = txt::parse(original_bytes, selection).map_err(TxtImportError::Parse)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let document = create_single_text_in(&transaction, "txt", filename, &parsed.text)?;
            transaction.execute(
                "INSERT INTO imported_files (document_id, sha256, original_bytes, text_encoding)
                 VALUES (?1, ?2, ?3, ?4)",
                (i64::from(document.document_id), digest.as_slice(), original_bytes, encoding_name(parsed.encoding)),
            ).map_err(StorageError::from)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let analysis = analyze(&document.chapter).map_err(TxtImportError::Analysis)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let imported = store_analyzed_document_in(
                &transaction,
                document,
                registered_at_unix_ms,
                &analysis,
            )?;
            TxtImportOutcome::Created(imported)
        };
        // Holding the cancellation gate through COMMIT makes its winner explicit:
        // cancellation rolls back, or a completed file is retained. This check is
        // after analysis, token writes and registrations, covering all partial data.
        cancellation.commit(|| {
            transaction.commit().map_err(StorageError::from)?;
            Ok(outcome)
        })
    }

    /// Read original bytes, selected encoding without parsing
    /// or loading the analyzer. None means no live TXT document with this ID.
    pub fn load_txt_source(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<StoredTxtSource>, StorageError> {
        let transaction = self.connection.transaction()?;
        let kind: Option<String> = transaction.query_row(
            "SELECT s.kind FROM documents d JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
            [i64::from(document_id)], |row| row.get(0),
        ).optional()?;
        if kind.as_deref() != Some("txt") {
            return Ok(None);
        }
        let source = transaction
            .query_row(
                "SELECT f.original_bytes, f.text_encoding, f.sha256 FROM imported_files f
             JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id
             WHERE d.id = ?1 AND s.kind = 'txt'",
                [i64::from(document_id)],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()?;
        let (original_bytes, encoding, digest) = source.ok_or(StorageError::InvalidDocument)?;
        if Sha256::digest(&original_bytes).as_slice() != digest {
            return Err(StorageError::InvalidDocument);
        }
        let encoding = match encoding.as_str() {
            "utf8" => TextEncoding::Utf8,
            "shift_jis" => TextEncoding::ShiftJis,
            _ => return Err(StorageError::InvalidDocument),
        };
        transaction.commit()?;
        Ok(Some(StoredTxtSource {
            original_bytes,
            encoding,
        }))
    }
}
