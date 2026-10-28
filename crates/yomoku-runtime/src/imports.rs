//! Worker-owned import analysis. Factories may cross threads; analyzers never do.

use std::{
    sync::{mpsc, Arc, Mutex, Weak},
    time::{SystemTime, UNIX_EPOCH},
};

use yomoku_core::{
    analysis::{AnalysisProfile, ChapterAnalysis, SplitMode},
    document::{ChapterText, PasteText, ReaderDocument},
    imports::{EpubImport, SrtImport, TextEncoding, TxtImport},
    storage::{DocumentError, StorageResult},
};
use yomoku_import::{
    epub, srt,
    txt::{self, TextEncoding as Decoding, TxtError},
};
use yomoku_storage::{
    EpubImportError, EpubImportOutcome, ImportCancellation, PasteImportError, PasteImportOutcome,
    SrtImportError, SrtImportOutcome, Storage, StorageError, TxtImportError, TxtImportOutcome,
};

use crate::storage::{map_error, Job};

/// A ready analyzer, created, reused and dropped exclusively on the FIFO worker.
/// Do not perform database work or publish provisional chapter IDs in `analyze`.
/// A closure capturing a Sudachi Analyzer implements this without requiring Send.
pub trait PasteAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError>;

    /// Explicit alternate generation only. Implementations must honor the pinned
    /// import profile; a newer engine/dictionary cannot reinterpret old content.
    /// Existing import-only factories remain valid and reject this capability.
    fn analyze_mode(
        &mut self,
        _chapter: &ChapterText,
        _mode: SplitMode,
        _profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        Err(DocumentError::Unavailable)
    }
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

/// FIFO worker handle: queued cache policy/image reads and direct file cancellation.
/// Keeping this handle alive does not retain a closed bridge's worker or callback.
pub struct AnalyzerLifecycle {
    pub(crate) sender: Weak<mpsc::Sender<Job>>,
    pub(crate) file: Weak<FileImportControl>,
}

impl AnalyzerLifecycle {
    /// Compatibility entry point for existing TXT callers.
    pub fn cancel_txt_import(&self) -> std::io::Result<()> {
        self.cancel_file_import()
    }

    /// Cancel the currently accepted EPUB/TXT/SRT attempt directly, never through FIFO.
    /// Call off the UI thread: an in-flight COMMIT can hold the per-file gate.
    /// Stop batch submission until this call and the import result both finish.
    /// A successful commit remains Created/Existing; idle calls are harmless.
    pub fn cancel_file_import(&self) -> std::io::Result<()> {
        self.file.upgrade().ok_or_else(worker_closed)?.cancel();
        Ok(())
    }

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

/// Published synchronously before enqueue, so cancellation also reaches work
/// waiting behind another FIFO job. Never hold this registry while taking the
/// per-file commit gate or delivering callbacks.
#[derive(Default)]
pub(crate) struct FileImportControl(Mutex<Option<Arc<ImportCancellation>>>);

impl FileImportControl {
    pub(crate) fn begin(&self) -> Arc<ImportCancellation> {
        let attempt = Arc::new(ImportCancellation::default());
        *self.0.lock().unwrap_or_else(|error| error.into_inner()) = Some(attempt.clone());
        attempt
    }

    pub(crate) fn finish(&self, attempt: &Arc<ImportCancellation>) {
        let mut current = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if current
            .as_ref()
            .is_some_and(|value| Arc::ptr_eq(value, attempt))
        {
            *current = None;
        }
    }

    fn cancel(&self) {
        let attempt = self
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(attempt) = attempt {
            attempt.cancel();
        }
    }
}

pub(crate) fn worker_closed() -> std::io::Error {
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

    pub(crate) fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        let result = self
            .analyzer
            .as_mut()
            .expect("loaded analyzer")
            .analyze_mode(chapter, mode, profile);
        if result.is_err() {
            self.analyzer = None;
        }
        result
    }

    pub(crate) fn discard_analyzer(&mut self) {
        self.analyzer = None;
    }

    pub(crate) fn import_txt(
        &mut self,
        storage: &mut Storage,
        input: &TxtImport,
        cancellation: &ImportCancellation,
    ) -> Result<StorageResult, DocumentError> {
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        // Avoid dictionary extraction/loading for an existing file, including
        // after background release or a fresh session. No title/encoding changes.
        if let Some(document) = storage
            .find_imported_file(&input.original_bytes)
            .map_err(map_error)?
        {
            return Ok(if cancellation.is_cancelled() {
                StorageResult::ImportCancelled
            } else {
                StorageResult::Existing(document)
            });
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
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        let analyzer = self.analyzer.as_mut().expect("loaded analyzer");
        match storage.import_analyzed_txt_cancellable(
            &input.original_bytes,
            &input.filename,
            selection,
            registration_timestamp()?,
            cancellation,
            |chapter| analyzer.analyze(chapter),
        ) {
            Ok(None) => Ok(StorageResult::ImportCancelled),
            Ok(Some(TxtImportOutcome::Created(imported))) => Ok(StorageResult::Created(
                imported
                    .document
                    .library_summary()
                    .map_err(|_| DocumentError::InvalidDocument)?,
            )),
            Ok(Some(TxtImportOutcome::Existing(document))) => Ok(StorageResult::Existing(document)),
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

    pub(crate) fn import_srt(
        &mut self,
        storage: &mut Storage,
        input: &SrtImport,
        cancellation: &ImportCancellation,
    ) -> Result<StorageResult, DocumentError> {
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        // Avoid dictionary extraction/loading for an existing file, including
        // after background release or a fresh session. No title/encoding changes.
        if let Some(document) = storage
            .find_imported_file(&input.original_bytes)
            .map_err(map_error)?
        {
            return Ok(if cancellation.is_cancelled() {
                StorageResult::ImportCancelled
            } else {
                StorageResult::Existing(document)
            });
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
        {
            let (_, decoded) =
                txt::decode(&input.original_bytes, selection).map_err(map_srt_decode_error)?;
            srt::parse(&decoded).map_err(|_| DocumentError::EmptySrt)?;
        }
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        let analyzer = self.analyzer.as_mut().expect("loaded analyzer");
        match storage.import_analyzed_srt_cancellable(
            &input.original_bytes,
            &input.filename,
            selection,
            registration_timestamp()?,
            cancellation,
            |chapter| analyzer.analyze(chapter),
        ) {
            Ok(None) => Ok(StorageResult::ImportCancelled),
            Ok(Some(SrtImportOutcome::Created(imported))) => Ok(StorageResult::Created(
                imported
                    .document
                    .library_summary()
                    .map_err(|_| DocumentError::InvalidDocument)?,
            )),
            Ok(Some(SrtImportOutcome::Existing(document))) => Ok(StorageResult::Existing(document)),
            Err(SrtImportError::Decode(error)) => Err(map_srt_decode_error(error)),
            Err(SrtImportError::Empty(_)) => Err(DocumentError::EmptySrt),
            Err(SrtImportError::Analysis(error)) => {
                self.analyzer = None;
                Err(error)
            }
            Err(SrtImportError::Storage(StorageError::InvalidAnalysis)) => {
                self.analyzer = None;
                Err(DocumentError::InvalidDocument)
            }
            Err(SrtImportError::Storage(error)) => Err(map_error(error)),
        }
    }

    pub(crate) fn import_epub(
        &mut self,
        storage: &mut Storage,
        input: &EpubImport,
        cancellation: &ImportCancellation,
    ) -> Result<StorageResult, DocumentError> {
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        // Exact original bytes win before filename/container validation or Full.
        if let Some(document) = storage
            .find_imported_file(&input.original_bytes)
            .map_err(map_error)?
        {
            return Ok(if cancellation.is_cancelled() {
                StorageResult::ImportCancelled
            } else {
                StorageResult::Existing(document)
            });
        }
        if input.filename.trim().is_empty() {
            return Err(DocumentError::InvalidDocument);
        }
        // Bound and validate the container before loading Full/taking the write
        // lock. Drop this projection; the repository repeats its authoritative
        // duplicate check and parse inside the whole-book transaction.
        epub::parse(&input.original_bytes).map_err(|_| DocumentError::InvalidEpub)?;
        if cancellation.is_cancelled() {
            return Ok(StorageResult::ImportCancelled);
        }
        if self.analyzer.is_none() {
            self.analyzer = Some((self.loader)()?);
        }
        let analyzer = self.analyzer.as_mut().expect("loaded analyzer");
        match storage.import_analyzed_epub_cancellable(
            &input.original_bytes,
            &input.filename,
            registration_timestamp()?,
            cancellation,
            |chapter| analyzer.analyze(chapter),
        ) {
            Ok(None) => Ok(StorageResult::ImportCancelled),
            Ok(Some(EpubImportOutcome::Created(imported))) => {
                let stored = imported.document;
                ReaderDocument {
                    document_id: stored.document_id,
                    title: stored.title,
                    finished_at_least_once: stored.finished_at_least_once,
                    reading_position: stored.reading_position,
                    formatting: Vec::new(),
                    images: Vec::new(),
                    source_ruby: Vec::new(),
                    chapters: stored
                        .chapters
                        .into_iter()
                        .map(|item| item.chapter)
                        .collect(),
                }
                .library_summary()
                .map(StorageResult::Created)
                .map_err(|_| DocumentError::InvalidDocument)
            }
            Ok(Some(EpubImportOutcome::Existing(document))) => {
                Ok(StorageResult::Existing(document))
            }
            Err(EpubImportError::Parse(_)) => Err(DocumentError::InvalidEpub),
            Err(EpubImportError::Analysis(error)) => {
                self.analyzer = None;
                Err(error)
            }
            Err(EpubImportError::Storage(StorageError::InvalidAnalysis)) => {
                self.analyzer = None;
                Err(DocumentError::InvalidDocument)
            }
            Err(EpubImportError::Storage(error)) => Err(map_error(error)),
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
            Ok(PasteImportOutcome::Created(imported)) => Ok(StorageResult::Created(
                imported
                    .document
                    .library_summary()
                    .map_err(|_| DocumentError::InvalidDocument)?,
            )),
            Ok(PasteImportOutcome::Existing(document)) => Ok(StorageResult::Existing(
                document
                    .library_summary()
                    .map_err(|_| DocumentError::InvalidDocument)?,
            )),
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

fn map_srt_decode_error(error: TxtError) -> DocumentError {
    match error {
        TxtError::EncodingRequired => DocumentError::SrtEncodingRequired,
        TxtError::InvalidEncoding(Decoding::Utf8) => DocumentError::SrtInvalidUtf8,
        TxtError::InvalidEncoding(Decoding::ShiftJis) => DocumentError::SrtInvalidShiftJis,
        TxtError::UnsupportedEncoding => DocumentError::SrtUnsupportedEncoding,
        TxtError::Undecodable => DocumentError::SrtUndecodable,
        TxtError::EmptyText => DocumentError::EmptySrt,
    }
}
