//! Frozen export presentation. IDs are observations, never invocation authority.

use super::AnkiExportError;
use crate::storage::DocumentError;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiExportPhase {
    Prepared,
    Checking,
    CreateModel,
    ReuseModel(i64),
    Existing(Vec<i64>),
    Beginning,
    Ready,
    Invoking,
    Saving,
    NoteAdded(i64),
    ModelCreated(i64),
    NotSubmitted,
    NoteUncertain(Option<i64>),
    ModelUncertain(Option<i64>),
    ProviderFailed(AnkiExportError),
    StorageFailed(DocumentError),
}

/// Retained independently of popup/navigation changes. Terminal live phases
/// describe the last durably acknowledged outcome, not a fresh provider query.
/// Recovery remains a separate projection of current journal obligations.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiExportView {
    /// Session-local generation of the frozen intent; never restored or reused.
    pub preparation_id: u64,
    pub surface: String,
    pub deck_id: i64,
    pub phase: AnkiExportPhase,
}
