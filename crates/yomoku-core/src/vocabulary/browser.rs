//! Read-only vocabulary browser values. Storage owns matching and pagination.

use serde::{Deserialize, Serialize};

use super::VocabularyStatus;
use crate::document::VocabularyId;

pub const VOCABULARY_PAGE_SIZE: u64 = 100;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum VocabularySort {
    #[default]
    Newest,
    Frequency,
    LastSeen,
    Alphabetical,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct VocabularyQuery {
    /// Literal written prefix; no normalization, trimming or wildcard syntax.
    pub prefix: String,
    /// OR within this set, AND with the prefix. Empty means no matches.
    pub statuses: Vec<VocabularyStatus>,
    pub sort: VocabularySort,
}

impl Default for VocabularyQuery {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            statuses: vec![
                VocabularyStatus::New,
                VocabularyStatus::Learning,
                VocabularyStatus::Familiar,
                VocabularyStatus::Known,
                VocabularyStatus::Ignored,
            ],
            sort: VocabularySort::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct VocabularyRow {
    pub id: VocabularyId,
    pub surface: String,
    pub status: VocabularyStatus,
    pub registered_at_unix_ms: i64,
    /// Committed completed-reading occurrences, including repetitions/rereads.
    pub encountered_count: u64,
    /// Latest completed-reading date; registration and lookup do not count.
    pub last_seen_unix_ms: Option<i64>,
}

/// Count and rows share one read snapshot. Offsets apply to an unchanged query
/// and dataset; callers must restart pagination after relevant mutations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct VocabularyPage {
    pub matching_count: u64,
    pub offset: u64,
    pub rows: Vec<VocabularyRow>,
    pub next_offset: Option<u64>,
}
