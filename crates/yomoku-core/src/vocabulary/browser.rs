//! Vocabulary browser and retained bulk values. Storage owns matching and I/O.

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
    Oldest,
    FrequencyAscending,
    LastSeenAscending,
    AlphabeticalDescending,
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

/// Session-retained query and loaded prefix of its results. None count means
/// the first page has not succeeded; Some(0) is a successfully empty result.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct VocabularyState {
    pub query: VocabularyQuery,
    pub matching_count: Option<u64>,
    pub rows: Vec<VocabularyRow>,
    pub next_offset: Option<u64>,
    pub loading: bool,
    pub error: Option<crate::storage::DocumentError>,
    /// Sorted session selection, including matches outside the loaded prefix.
    pub selection: Option<Vec<VocabularyId>>,
    pub selecting_all: bool,
    pub confirmation: Option<VocabularyBulkCommand>,
    pub bulk_error: Option<crate::storage::DocumentError>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum VocabularyBulkAction {
    SetStatus {
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    },
    Delete,
}

/// Confirmation and Retry always use these exact IDs/action/date, never a query.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct VocabularyBulkCommand {
    pub ids: Vec<VocabularyId>,
    pub action: VocabularyBulkAction,
}

impl VocabularyQuery {
    /// A status filter is a set. Preserve literal spelling while making equal
    /// subsets independent of checkbox order or duplicate shell values.
    pub fn canonicalized(mut self) -> Self {
        self.statuses = VocabularyQuery::default()
            .statuses
            .into_iter()
            .filter(|status| self.statuses.contains(status))
            .collect();
        self
    }
}
