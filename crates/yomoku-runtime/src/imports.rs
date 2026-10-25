//! Worker-owned import analysis. Factories may cross threads; analyzers never do.

use std::{
    sync::{mpsc, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, PasteText},
    imports::{TextEncoding, TxtImport},
    storage::{DocumentError, StorageResult},
};
use yomoku_import::txt::{self, TextEncoding as Decoding, TxtError};
use yomoku_storage::{
    PasteImportError, PasteImportOutcome, Storage, StorageError, TxtImportError, TxtImportOutcome,
};

use crate::storage::{map_error, Job};

/// A ready analyzer, created, reused and dropped exclusively on the FIFO worker.
/// Do not perform database work or publish provisional chapter IDs in `analyze`.
/// A closure capturing a Sudachi Analyzer implements this without requiring Send.
pub trait PasteAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError>;
}

impl<F> PasteAnalyzer for F
where
    F: FnMut(&ChapterText) -> Result<ChapterAnalysis, DocumentError>,
{
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        self(chapter)
    }
}

/// Load/verify platform assets here, before SQLite starts the import transaction.
/// A failed load is retryable. The factory is lazy even when Library is refreshed.
pub type AnalyzerLoader = Box<dyn FnMut() -> Result<Box<dyn PasteAnalyzer>, DocumentError> + Send>;

/// Queues cache policy on the same FIFO as accepted storage operations. Keeping
/// this handle alive does not retain a closed bridge's worker or callback.
pub struct AnalyzerLifecycle {
    pub(crate) sender: Weak<mpsc::Sender<Job>>,
}

impl AnalyzerLifecycle {
    /// Background/UI-hidden releases the cache after earlier atomic operations.
    /// Imports accepted while hidden finish, then release their analyzer too.
    /// Foreground permits reuse, but never eagerly opens the dictionary.
    pub fn set_foreground(&self, foreground: bool) -> std::io::Result<()> {
        self.sender
            .upgrade()
            .ok_or_else(worker_closed)?
            .send(Job::Foreground(foreground))
            .map_err(|_| worker_closed())
    }
}

fn worker_closed() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "import worker closed")
}

pub(crate) struct Imports {
    loader: AnalyzerLoader,
    analyzer: Option<Box<dyn PasteAnalyzer>>,
    foreground: bool,
}

impl Imports {
    pub(crate) fn new(loader: AnalyzerLoader) -> Self {
        Self {
            loader,
            analyzer: None,
            foreground: true,
        }
    }

    pub(crate) fn set_foreground(&mut self, foreground: bool) {
        self.foreground = foreground;
        self.release_if_hidden();
    }

    pub(crate) fn release_if_hidden(&mut self) {
        if !self.foreground {
            self.analyzer = None;
        }
    }

    pub(crate) fn import_txt(
        &mut self,
        storage: &mut Storage,
        input: &TxtImport,
    ) -> Result<StorageResult, DocumentError> {
        // Avoid dictionary extraction/loading for an existing file, including
        // after background release or a fresh session. No title/encoding changes.
        if let Some(document) = storage
            .find_imported_txt(&input.original_bytes)
            .map_err(map_error)?
        {
            return Ok(StorageResult::Existing(document));
        }
        if input.filename.trim().is_empty() {
            return Err(DocumentError::InvalidDocument);
        }
        let selection = input.encoding.map(|encoding| match encoding {
            TextEncoding::Utf8 => Decoding::Utf8,
            TextEncoding::ShiftJis => Decoding::ShiftJis,
        });
        // Validate before loading Full and before acquiring the write lock.
        // The repository parses again under its authoritative atomic lookup so
        // concurrent creators cannot overwrite an existing interpretation.
        txt::parse(&input.original_bytes, selection).map_err(map_txt_error)?;
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        let analyzer = self.analyzer.as_mut().expect("loaded analyzer");
        match storage.import_analyzed_txt(
            &input.original_bytes,
            &input.filename,
            selection,
            registration_timestamp()?,
            |chapter| analyzer.analyze(chapter),
        ) {
            Ok(TxtImportOutcome::Created(imported)) => {
                Ok(StorageResult::Created(imported.document))
            }
            Ok(TxtImportOutcome::Existing(document)) => Ok(StorageResult::Existing(document)),
            Err(TxtImportError::Parse(error)) => Err(map_txt_error(error)),
            Err(TxtImportError::Analysis(error)) => {
                self.analyzer = None;
                Err(error)
            }
            Err(TxtImportError::Storage(StorageError::InvalidAnalysis)) => {
                self.analyzer = None;
                Err(DocumentError::InvalidDocument)
            }
            Err(TxtImportError::Storage(error)) => Err(map_error(error)),
        }
    }

    pub(crate) fn import(
        &mut self,
        storage: &mut Storage,
        paste: &PasteText,
    ) -> Result<StorageResult, DocumentError> {
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        // Wall time dates registration history; active reading uses a separate
        // monotonic clock and is never started/committed by an import.
        let timestamp = registration_timestamp()?;
        let analyzer = self.analyzer.as_mut().expect("loaded analyzer");
        let result =
            storage.import_analyzed_paste(paste, timestamp, |chapter| analyzer.analyze(chapter));
        match result {
            Ok(PasteImportOutcome::Created(imported)) => {
                Ok(StorageResult::Created(imported.document))
            }
            Ok(PasteImportOutcome::Existing(document)) => Ok(StorageResult::Existing(document)),
            Err(PasteImportError::Analysis(error)) => {
                // A later attempt loads fresh state after any engine failure.
                self.analyzer = None;
                Err(error)
            }
            Err(PasteImportError::Storage(StorageError::InvalidAnalysis)) => {
                self.analyzer = None;
                Err(DocumentError::InvalidDocument)
            }
            Err(PasteImportError::Storage(error)) => Err(map_error(error)),
        }
    }
}

fn registration_timestamp() -> Result<i64, DocumentError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or(DocumentError::Unavailable)
}

fn map_txt_error(error: TxtError) -> DocumentError {
    match error {
        TxtError::EncodingRequired => DocumentError::TxtEncodingRequired,
        TxtError::InvalidEncoding(Decoding::Utf8) => DocumentError::TxtInvalidUtf8,
        TxtError::InvalidEncoding(Decoding::ShiftJis) => DocumentError::TxtInvalidShiftJis,
        TxtError::UnsupportedEncoding => DocumentError::TxtUnsupportedEncoding,
        TxtError::Undecodable => DocumentError::TxtUndecodable,
        TxtError::EmptyText => DocumentError::EmptyTxt,
    }
}
