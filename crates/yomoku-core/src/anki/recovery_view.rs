//! Compact presentation only: never an authority to invoke a provider mutation.

use super::AnkiExportError;
use crate::storage::DocumentError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiRecoveryPhase {
    Loading,
    Ready,
    Unresolved,
    Checking,
    Saving,
    Busy,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiRecoveryView {
    pub phase: AnkiRecoveryPhase,
    pub note_pending: bool,
    pub model_pending: bool,
    pub storage_error: Option<DocumentError>,
    pub provider_error: Option<AnkiExportError>,
    pub can_retry: bool,
    pub can_check_note: bool,
    pub can_check_model: bool,
}
