//! Pure storage effect protocol. SQL, paths and execution belong to runtime.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use crate::active_time::{ReadingTimeUpdate, SavedReadingTime};
use crate::document::{DocumentId, LibraryDocument, PastedDocument};
use crate::pagination::{PageLength, ReadingPosition};
use crate::reader::ReaderSnapshot;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageOperation {
    List,
    CreatePaste(String),
    LoadPaste(DocumentId),
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StorageResult {
    Listed(Vec<LibraryDocument>, PageLength),
    Created(PastedDocument),
    /// Exact duplicate: preserve persisted title/position and reference it once.
    Existing(PastedDocument),
    Loaded(ReaderSnapshot),
    PositionSaved(PastedDocument),
    PageLengthSaved(PageLength),
    ReadingTimeLoaded(SavedReadingTime),
    ReadingTimeSaved(SavedReadingTime),
    Failed(DocumentError),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}
