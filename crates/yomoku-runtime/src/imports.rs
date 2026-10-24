//! Worker-owned import analysis. Factories may cross threads; analyzers never do.

use std::{
    sync::{mpsc, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, PasteText},
    storage::{DocumentError, StorageResult},
};
use yomoku_storage::{PasteImportError, PasteImportOutcome, Storage, StorageError};

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
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .ok_or(DocumentError::Unavailable)?;
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
