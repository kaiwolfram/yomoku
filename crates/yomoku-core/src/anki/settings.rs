//! Local preference presentation. A configured ID grants no provider authority.

use crate::storage::DocumentError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiSettingsPhase {
    #[default]
    Unloaded,
    Loading,
    Ready,
    Saving,
    Failed,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiSettingsView {
    pub phase: AnkiSettingsPhase,
    /// Last acknowledged preference; meaningful for new actions only when Ready.
    pub default_deck_id: Option<i64>,
    pub storage_error: Option<DocumentError>,
}
