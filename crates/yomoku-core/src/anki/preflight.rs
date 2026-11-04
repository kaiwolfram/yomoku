//! Read-only observations for a frozen export. None of these values grants a
//! live mutation token; durable begin and fresh provider checks are still needed.

use std::{collections::BTreeSet, sync::Arc};

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use super::{AnkiExistingNotes, AnkiExport, AnkiExportError, AnkiModelId, AnkiReadModelLayout};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiPreflightDeck {
    pub id: i64,
    pub filtered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiPreflightModel {
    pub id: i64,
    pub name: String,
    /// Complete validation against the request's frozen layout, including normal
    /// type, sort field, ordered fields, CSS, forward template and browser overrides.
    /// A failed/partial validation must fail the whole observation, never use false.
    pub matches_layout: bool,
}

/// Supply only after all provider/spec/permission, deck, exact-surface duplicate
/// and model reads succeed. Lists must be complete; discard all partial results
/// on any read failure. Duplicate IDs must have verified exact first fields,
/// across all models/decks. No inferred absence from failed queries is allowed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiPreflightObservation {
    pub decks: Vec<AnkiPreflightDeck>,
    pub duplicate_note_ids: Vec<i64>,
    pub models: Vec<AnkiPreflightModel>,
}

/// Read-only transport. The native request/selection and prepared note fields
/// stay in core; this data cannot grant provider write permission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiPreflightOperation {
    /// Empty only for Settings/startup model validation; never a note identity.
    pub surface: String,
    pub deck_id: i64,
    pub layout: Box<AnkiReadModelLayout>,
}

impl Operation for AnkiPreflightOperation {
    type Output = AnkiPreflightResult;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiPreflightResult {
    Complete(AnkiPreflightObservation),
    Failed(AnkiExportError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiPreflightPlan {
    Existing(AnkiExistingNotes),
    ReuseModel(AnkiModelId),
    CreateModel,
}

/// Capture before reading. Shared allocation identity distinguishes retries,
/// equal exports in another owner and clear/reprepare cycles. Never serialize
/// this handle or reconstruct it from shell/provider values.
#[derive(Clone, Debug)]
pub struct AnkiPreflightRequest(Arc<AnkiExport>);

impl PartialEq for AnkiPreflightRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AnkiPreflightRequest {}

impl AnkiPreflightRequest {
    pub(crate) fn new(export: AnkiExport) -> Self {
        Self(Arc::new(export))
    }

    pub fn export(&self) -> &AnkiExport {
        &self.0
    }

    pub fn operation(&self) -> AnkiPreflightOperation {
        AnkiPreflightOperation {
            surface: self.export().note().surface().as_str().to_owned(),
            deck_id: self.export().deck().into(),
            layout: Box::new(self.export().layout().into()),
        }
    }

    pub(crate) fn evaluate(
        &self,
        observation: AnkiPreflightObservation,
    ) -> Result<AnkiPreflightPlan, AnkiExportError> {
        evaluate_layout(
            self.export().layout(),
            self.export().deck().into(),
            observation,
        )
    }
}

pub(crate) fn evaluate_layout(
    layout: &super::AnkiModelLayout,
    deck_id: i64,
    observation: AnkiPreflightObservation,
) -> Result<AnkiPreflightPlan, AnkiExportError> {
    let invalid = AnkiExportError::InvalidResponse;
    let mut deck_ids = BTreeSet::new();
    for deck in &observation.decks {
        if deck.id <= 0 || !deck_ids.insert(deck.id) {
            return Err(invalid);
        }
    }
    let mut model_ids = BTreeSet::new();
    for model in &observation.models {
        if model.id <= 0
            || model.name.is_empty()
            || !model_ids.insert(model.id)
            || (model.matches_layout && model.name != layout.name)
        {
            return Err(invalid);
        }
    }
    let notes = observation
        .duplicate_note_ids
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid)?;
    if !observation
        .decks
        .iter()
        .any(|deck| deck.id == deck_id && !deck.filtered)
    {
        return Err(AnkiExportError::DeckUnavailable);
    }
    if !notes.is_empty() {
        return Ok(AnkiPreflightPlan::Existing(
            AnkiExistingNotes::new(notes).map_err(|_| invalid)?,
        ));
    }
    let mut named = observation
        .models
        .iter()
        .filter(|model| model.name == layout.name);
    match (named.next(), named.next()) {
        (None, _) => Ok(AnkiPreflightPlan::CreateModel),
        (Some(model), None) if model.matches_layout => Ok(AnkiPreflightPlan::ReuseModel(
            model.id.try_into().map_err(|_| invalid)?,
        )),
        // Never pick an arbitrary same-name model, rename or overwrite it.
        _ => Err(AnkiExportError::IncompatibleModel),
    }
}

pub(crate) struct AnkiPreflight {
    pub request: AnkiPreflightRequest,
    pub result: Option<Result<AnkiPreflightPlan, AnkiExportError>>,
}
