//! Entry identities are scoped to a current native reader selection.
use super::{AnkiExportError, AnkiExportPhase, AnkiSelection};
use crate::dictionary::{DictionaryCandidates, ReadingMatch};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiEntryKey {
    Word {
        entry_id: i64,
        reading: ReadingMatch,
    },
    Name {
        entry_id: i64,
        reading: ReadingMatch,
    },
}

impl AnkiEntryKey {
    /// Surface/base/component matches can repeat a pair; keep its first occurrence.
    pub fn for_candidates(candidates: &DictionaryCandidates) -> Vec<Self> {
        let mut keys = Vec::new();
        for candidate in &candidates.words {
            for found in candidate.forms.iter().flat_map(|form| &form.matches) {
                let key = Self::Word {
                    entry_id: candidate.entry_id,
                    reading: found.reading,
                };
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        for candidate in &candidates.names {
            for reading in candidate.forms.iter().flat_map(|form| &form.matches) {
                let key = Self::Name {
                    entry_id: candidate.entry_id,
                    reading: *reading,
                };
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        keys
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiEntryPhase {
    Checking,
    Add,
    Existing(Vec<i64>),
    Export(AnkiExportPhase),
    Failed(AnkiExportError),
    Busy,
}

/// Only an explicit Settings request may configure a missing model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiSetupRequest {
    Refresh,
    Configure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiEntryView {
    pub selection: AnkiSelection,
    pub key: AnkiEntryKey,
    pub headword: String,
    pub reading: String,
    pub phase: AnkiEntryPhase,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiSetupPhase {
    #[default]
    Unchecked,
    Checking,
    Missing,
    Ready,
    Failed(AnkiExportError),
}
