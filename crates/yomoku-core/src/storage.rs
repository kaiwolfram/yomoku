//! Pure storage effect protocol. SQL, paths and execution belong to runtime.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::document::{DocumentId, LibraryDocument, ReaderDocument};
use crate::effective_storage::{EffectiveStorageOperation, EffectiveStorageResult};
use crate::imports::{EpubImport, SrtImport, TxtImport};
use crate::library::LibraryWordDistribution;
use crate::pagination::{PageLength, ReadingPosition};
use crate::reader::DocumentReaderSnapshot;
use crate::typography::ReaderTypography;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageOperation {
    List,

    DeleteDocument(DocumentId),
    RenameDocument {
        document_id: DocumentId,
        title: String,
    },
    CreatePaste(String),

    /// Explicit entry records Library recency and resets a finished book atomically.
    OpenReader(DocumentId),
    SaveReadingPosition {
        document_id: DocumentId,
        position: ReadingPosition,
    },
    SavePageLength(PageLength),
    SaveReaderTypography(ReaderTypography),
    ImportTxt(TxtImport),
    ImportSrt(SrtImport),
    ImportEpub(EpubImport),
    /// Native FIFO read. Only the compact chart values in ViewModel cross FFI.
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    LoadStatistics(crate::statistics::StatisticsQuery),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    LoadVocabulary {
        query: crate::vocabulary::VocabularyQuery,
        offset: u64,
    },
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyMatchingIds(crate::vocabulary::VocabularyQuery),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    ApplyVocabularyBulk(crate::vocabulary::VocabularyBulkCommand),
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

    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    LoadAnkiSettings,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    SaveAnkiDefaultDeck(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Option<crate::anki::AnkiDeckId>,
    ),
}

impl StorageOperation {
    pub(crate) fn loads_library_words(&self) -> bool {
        matches!(self, Self::LoadEffectiveLibraryWordDistributions)
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

    Failed(DocumentError),

    /// Cooperative file cancellation rolled back the current file, without error.
    ImportCancelled,
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    StatisticsLoaded(Box<crate::statistics::StatisticsBuckets>),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyLoaded(crate::vocabulary::VocabularyPage),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyMatchingIds(Vec<crate::document::VocabularyId>),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    VocabularyBulkApplied(crate::vocabulary::VocabularyBulkCommand),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    Effective(#[cfg_attr(feature = "facet_typegen", facet(opaque))] EffectiveStorageResult),

    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiSettingsLoaded(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Option<crate::anki::AnkiDeckId>,
    ),
    #[serde(skip)]
    #[cfg_attr(feature = "facet_typegen", facet(skip))]
    AnkiDefaultDeckSaved(
        #[cfg_attr(feature = "facet_typegen", facet(opaque))] Option<crate::anki::AnkiDeckId>,
    ),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}
