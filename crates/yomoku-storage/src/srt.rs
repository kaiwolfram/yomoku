//! Atomic original SRT bytes, canonical text and analyzed registration.

use std::{error::Error, fmt};

use rusqlite::TransactionBehavior;
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, LibraryDocument},
};
use yomoku_import::{
    srt::{self, EmptySrt},
    txt::{decode, TextEncoding, TxtError},
};

use crate::{
    documents::create_single_text_in,
    files::{find_imported_in, store_imported_text_in},
    imports::store_analyzed_document_in,
    AnalyzedPaste, ImportCancellation, Storage, StorageError,
};

#[derive(Debug)]
pub enum SrtImportOutcome {
    Created(AnalyzedPaste),
    Existing(LibraryDocument),
}

#[derive(Debug)]
pub enum SrtImportError<E> {
    /// Uses the shared strict text decoder; no Aozora transformation is applied.
    Decode(TxtError),
    Empty(EmptySrt),
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for SrtImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "SRT decoding failed: {error}"),
            Self::Empty(error) => error.fmt(f),
            Self::Analysis(error) => write!(f, "SRT analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for SrtImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Empty(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for SrtImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl Storage {
    /// Hash before decoding. Exact live duplicates retain their interpretation,
    /// title, progress and statuses without invoking the analyzer. A new SRT
    /// uses its literal display filename as title and one canonical chapter.
    /// The ready worker callback must not publish provisional IDs or do DB I/O.
    pub fn import_analyzed_srt<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<SrtImportOutcome, SrtImportError<E>> {
        self.import_analyzed_srt_cancellable(
            original_bytes,
            filename,
            selection,
            registered_at_unix_ms,
            &ImportCancellation::default(),
            analyze,
        )
        .map(|outcome| outcome.expect("unshared import cannot be cancelled"))
    }

    /// Share a fresh handle before worker submission and request cancellation
    /// outside that worker. None discards this file, including allocated IDs;
    /// Some retains a winning commit. Prior files are never affected.
    pub fn import_analyzed_srt_cancellable<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        cancellation: &ImportCancellation,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<Option<SrtImportOutcome>, SrtImportError<E>> {
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
        let outcome = if let Some(document) = find_imported_in(&transaction, digest.as_slice())? {
            SrtImportOutcome::Existing(document)
        } else {
            if filename.trim().is_empty() {
                return Err(StorageError::InvalidDocument.into());
            }
            let (encoding, decoded) =
                decode(original_bytes, selection).map_err(SrtImportError::Decode)?;
            let parsed = srt::parse(&decoded).map_err(SrtImportError::Empty)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let document = create_single_text_in(&transaction, "srt", filename, &parsed.text)?;
            store_imported_text_in(
                &transaction,
                document.document_id,
                digest.as_slice(),
                encoding,
                &parsed.text,
            )?;
            let analysis = analyze(&document.chapter).map_err(SrtImportError::Analysis)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            SrtImportOutcome::Created(store_analyzed_document_in(
                &transaction,
                document,
                registered_at_unix_ms,
                &analysis,
            )?)
        };
        cancellation.commit(|| {
            transaction.commit().map_err(StorageError::from)?;
            Ok(outcome)
        })
    }
}
