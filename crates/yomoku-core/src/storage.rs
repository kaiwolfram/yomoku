//! Pure storage effect protocol. SQL, paths and execution belong to runtime.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::active_time::{ReadingTimeUpdate, SavedReadingTime};
use crate::completion::{AnalyzedCompletionOutcome, AnalyzedPageCompletion, PageCompletionIntent};
use crate::document::{DocumentId, LibraryDocument, ReaderDocument};
use crate::imports::{SrtImport, TxtImport};
use crate::pagination::{PageLength, ReadingPosition};
use crate::reader::{
    DocumentReaderSnapshot, ReaderStatusChange, ReaderTokenKey, ReaderTokenSnapshot,
    ReaderVocabulary, ReaderWordDetails, ReaderWordSelection,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageOperation {
    List,
    CreatePaste(String),
    LoadPaste(DocumentId),
    /// Explicit reader entry; resets a finished document to page 1 atomically.
    OpenReader(DocumentId),
    SaveReadingPosition {
        document_id: DocumentId,
        position: ReadingPosition,
    },
    SavePageLength(PageLength),
    LoadReadingTime {
        document_id: DocumentId,
        position: ReadingPosition,
    },
    SaveReadingTime {
        previous: SavedReadingTime,
        checkpoint: ReadingTimeUpdate,
    },
    PreparePageCompletion(PageCompletionIntent),
    CompletePage(AnalyzedPageCompletion),
    /// Read original tokens and live global statuses without registration.
    LoadReaderTokens(ReaderTokenKey),
    SetReaderStatus(ReaderStatusChange),
    LoadReaderWord(ReaderWordSelection),
    ImportTxt(TxtImport),
    ImportSrt(SrtImport),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum DocumentError {
    EmptyPaste,
    Unavailable,
    IncompatibleDatabase,
    InvalidDocument,
    NotFound,
    InvalidSettings,
    MissingAnalysis,
    TxtEncodingRequired,
    TxtInvalidUtf8,
    TxtInvalidShiftJis,
    TxtUnsupportedEncoding,
    TxtUndecodable,
    EmptyTxt,
    SrtEncodingRequired,
    SrtInvalidUtf8,
    SrtInvalidShiftJis,
    SrtUnsupportedEncoding,
    SrtUndecodable,
    EmptySrt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageResult {
    Listed(Vec<LibraryDocument>, PageLength),
    /// Import results reference Library state; content is loaded separately.
    Created(LibraryDocument),
    /// Exact duplicate: preserve persisted title/position and reference it once.
    Existing(LibraryDocument),
    Loaded(DocumentReaderSnapshot),
    PositionSaved(ReaderDocument),
    PageLengthSaved(PageLength),
    ReadingTimeLoaded(SavedReadingTime),
    ReadingTimeSaved(SavedReadingTime),
    PageCompletionPrepared(AnalyzedPageCompletion),
    PageCompleted(ReaderDocument, AnalyzedCompletionOutcome),
    Failed(DocumentError),
    ReaderTokensLoaded(ReaderTokenSnapshot),
    ReaderWordLoaded(Box<ReaderWordDetails>),
    ReaderStatusChanged(ReaderStatusChange, Option<ReaderVocabulary>),
    /// Cooperative file cancellation rolled back the current file, without error.
    ImportCancelled,
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}
