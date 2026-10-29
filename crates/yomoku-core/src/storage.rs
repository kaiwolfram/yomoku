//! Pure storage effect protocol. SQL, paths and execution belong to runtime.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::active_time::{ReadingTimeUpdate, SavedReadingTime};
use crate::completion::{AnalyzedCompletionOutcome, AnalyzedPageCompletion, PageCompletionIntent};
use crate::document::{DocumentId, LibraryDocument, ReaderDocument};
use crate::effective_storage::{EffectiveStorageOperation, EffectiveStorageResult};
use crate::imports::{EpubImport, SrtImport, TxtImport};
use crate::library::LibraryWordDistribution;
use crate::pagination::{PageLength, ReadingPosition};
use crate::reader::{
    DocumentReaderSnapshot, ReaderCustomEntry, ReaderStatusChange, ReaderTokenKey,
    ReaderTokenSnapshot, ReaderVocabulary, ReaderWordDetails, ReaderWordSelection,
};
use crate::typography::ReaderTypography;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageOperation {
    List,
    LoadLibraryWordDistributions,
    DeleteDocument(DocumentId),
    DeleteDocuments(Vec<DocumentId>),
    RenameDocument {
        document_id: DocumentId,
        title: String,
    },
    CreatePaste(String),
    LoadPaste(DocumentId),
    /// Explicit entry records Library recency and resets a finished book atomically.
    OpenReader(DocumentId),
    SaveReadingPosition {
        document_id: DocumentId,
        position: ReadingPosition,
    },
    SavePageLength(PageLength),
    SaveReaderTypography(ReaderTypography),
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
    SaveReaderCustomEntry {
        selection: ReaderWordSelection,
        english_meaning: String,
        reading: String,
    },
    DeleteReaderCustomEntry(ReaderWordSelection),
    ImportTxt(TxtImport),
    ImportSrt(SrtImport),
    ImportEpub(EpubImport),
    /// Native staging read, using the same compact result and app cache gates
    /// as original B. Activate production with the effective reader consumers.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    LoadEffectiveLibraryWordDistributions,
    /// Native-only protocol. Neither full keys nor reserved completions have a
    /// shell codec; the generated event/view schemas remain compact.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    Effective(#[cfg_attr(feature = "facet_typegen", facet(opaque))] EffectiveStorageOperation),
}

impl StorageOperation {
    pub(crate) fn loads_library_words(&self) -> bool {
        matches!(
            self,
            Self::LoadLibraryWordDistributions | Self::LoadEffectiveLibraryWordDistributions
        )
    }
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
    InvalidEpub,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageResult {
    Listed(Vec<LibraryDocument>, PageLength, ReaderTypography),
    LibraryWordDistributionsLoaded(Vec<LibraryWordDistribution>),
    DocumentDeleted(DocumentId),
    DocumentsDeleted(Vec<DocumentId>),
    DocumentRenamed {
        document_id: DocumentId,
        title: String,
    },
    /// Import results reference Library state; content is loaded separately.
    Created(LibraryDocument),
    /// Exact duplicate: preserve persisted title/position and reference it once.
    Existing(LibraryDocument),
    Loaded(DocumentReaderSnapshot),
    PositionSaved(ReaderDocument),
    PageLengthSaved(PageLength),
    ReaderTypographySaved(ReaderTypography),
    ReadingTimeLoaded(SavedReadingTime),
    ReadingTimeSaved(SavedReadingTime),
    PageCompletionPrepared(AnalyzedPageCompletion),
    PageCompleted(ReaderDocument, AnalyzedCompletionOutcome),
    Failed(DocumentError),
    ReaderTokensLoaded(ReaderTokenSnapshot),
    ReaderWordLoaded(Box<ReaderWordDetails>),
    ReaderCustomEntryChanged(ReaderWordSelection, Option<ReaderCustomEntry>),
    ReaderStatusChanged(ReaderStatusChange, Option<ReaderVocabulary>),
    /// Cooperative file cancellation rolled back the current file, without error.
    ImportCancelled,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    Effective(#[cfg_attr(feature = "facet_typegen", facet(opaque))] EffectiveStorageResult),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}
