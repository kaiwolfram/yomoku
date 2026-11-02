//! Pure Library presentation. Import identity is durable commit order, not wall time.

use crate::document::{DocumentId, LibraryDocument};
use serde::{Deserialize, Serialize};

/// Session-local title editor. Only an explicit save changes the live title.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct LibraryRenameState {
    pub document_id: DocumentId,
    pub title: String,
    pub error: Option<crate::storage::DocumentError>,
}

pub fn valid_document_title(title: &str) -> bool {
    !title.trim().is_empty()
}

/// Registered exact surfaces, counted once per entry in a Library projection or
/// historical instant. Missing/deleted vocabulary is absent, never implicitly New.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct WordStatusCounts {
    pub new: u64,
    pub learning: u64,
    pub familiar: u64,
    pub known: u64,
    pub ignored: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct LibraryWordDistribution {
    pub document_id: DocumentId,
    /// None when any chapter lacks its original B analysis. Some(all zero) is
    /// an analyzed document with no remaining registered Japanese surfaces.
    pub counts: Option<WordStatusCounts>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
// Default directions are newest first for dates/recency and ascending for titles.
// Reverse variants reverse the full stable order, including document-ID ties.
pub enum LibrarySort {
    #[default]
    LastRead,
    Title,
    ImportDate,
    LastReadAscending,
    TitleDescending,
    ImportDateAscending,
}

impl LibrarySort {
    pub fn project(self, documents: &[LibraryDocument], query: &str) -> Vec<LibraryDocument> {
        let query = query.to_lowercase();
        // Decorate once, so case folding is not repeated for every comparison.
        let mut matching: Vec<_> = documents
            .iter()
            .filter_map(|document| {
                let title = document.title.to_lowercase();
                (query.is_empty() || title.contains(&query)).then_some((document, title))
            })
            .collect();
        matching.sort_by(|(left, left_title), (right, right_title)| {
            match self {
                Self::LastRead | Self::LastReadAscending => {
                    right.last_read_sequence.cmp(&left.last_read_sequence)
                }
                Self::Title | Self::TitleDescending => left_title.cmp(right_title),
                Self::ImportDate | Self::ImportDateAscending => std::cmp::Ordering::Equal,
            }
            .then_with(|| right.document_id.cmp(&left.document_id))
        });
        if matches!(
            self,
            Self::LastReadAscending | Self::TitleDescending | Self::ImportDateAscending
        ) {
            matching.reverse();
        }
        matching
            .into_iter()
            .map(|(document, _)| document.clone())
            .collect()
    }
}
