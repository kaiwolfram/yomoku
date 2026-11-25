//! Atomic original-file identity, canonical TXT content.

use std::{error::Error, fmt};

use rusqlite::{OptionalExtension, TransactionBehavior};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, DocumentId, LibraryDocument},
};
use yomoku_import::txt::{self, TextEncoding, TxtError};

use crate::{
    documents::create_single_text_in,
    files::{find_imported_in, store_imported_text_in},
    imports::store_analyzed_document_in,
    AnalyzedPaste, FilePreflight, ImportCancellation, PreparedFile, Storage, StorageError,
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

impl Storage {
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
        match self.prepare_txt(original_bytes, filename, selection)? {
            FilePreflight::Existing(document) => {
                cancellation.commit(|| Ok(TxtImportOutcome::Existing(document)))
            }
            FilePreflight::Prepared(prepared) => {
                self.import_prepared_txt(prepared, registered_at_unix_ms, cancellation, analyze)
            }
        }
    }

    pub fn prepare_txt<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
    ) -> Result<FilePreflight<txt::TxtDocument>, TxtImportError<E>> {
        self.prepare_file(original_bytes, filename, |bytes| {
            txt::parse(bytes, selection).map_err(TxtImportError::Parse)
        })
    }

    pub fn import_prepared_txt<E>(
        &mut self,
        prepared: PreparedFile<txt::TxtDocument>,
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
        let PreparedFile {
            digest,
            filename,
            parsed,
        } = prepared;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let outcome = if let Some(document) = find_imported_in(&transaction, digest.as_slice())? {
            TxtImportOutcome::Existing(document)
        } else {
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let document = create_single_text_in(&transaction, "txt", &filename, &parsed.text)?;
            store_imported_text_in(
                &transaction,
                document.document_id,
                digest.as_slice(),
                parsed.encoding,
                &parsed.text,
            )?;
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

    /// Read the selected decoder for a live TXT document.
    pub fn load_txt_encoding(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<TextEncoding>, StorageError> {
        let encoding: Option<Option<String>> = self
            .connection
            .query_row(
                "SELECT f.text_encoding FROM documents d
                 JOIN sources s ON s.id = d.source_id
                 LEFT JOIN imported_files f ON f.document_id = d.id
                 WHERE d.id = ?1 AND s.kind = 'txt'",
                [i64::from(document_id)],
                |row| row.get(0),
            )
            .optional()?;
        match encoding {
            None => Ok(None),
            Some(encoding) => match encoding.as_deref() {
                Some("utf8") => Ok(Some(TextEncoding::Utf8)),
                Some("shift_jis") => Ok(Some(TextEncoding::ShiftJis)),
                _ => Err(StorageError::InvalidDocument),
            },
        }
    }
}
