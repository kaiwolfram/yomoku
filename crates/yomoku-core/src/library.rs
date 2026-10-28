//! Pure Library presentation. Import identity is durable commit order, not wall time.

use crate::document::LibraryDocument;
use serde::{Deserialize, Serialize};

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
