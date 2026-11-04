//! Entry identities are scoped to a current native reader selection.
use super::{AnkiExportError, AnkiExportPhase, AnkiSelection};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiEntryKey {
    Word(i64),
    Name(i64),
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
    RecoveryRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiEntryView {
    pub selection: AnkiSelection,
    pub key: AnkiEntryKey,
    pub headword: String,
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
