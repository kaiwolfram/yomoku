//! Pure Library presentation. Import identity is durable commit order, not wall time.

use crate::document::{DocumentId, LibraryDocument};
use serde::{Deserialize, Serialize};

/// Live registered exact surfaces, each counted once under the chosen document
/// segmentation. Missing/deleted vocabulary is absent, never implicitly New.
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
pub enum LibrarySort {
    #[default]
    LastRead,
    Title,
    ImportDate,
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
                Self::LastRead => right.last_read_sequence.cmp(&left.last_read_sequence),
                Self::Title => left_title.cmp(right_title),
                Self::ImportDate => std::cmp::Ordering::Equal,
            }
            .then_with(|| right.document_id.cmp(&left.document_id))
        });
        matching
            .into_iter()
            .map(|(document, _)| document.clone())
            .collect()
    }
}
