//! Native read handles retain provenance, never permission to mutate a provider.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{AnkiExportError, AnkiModelLayout, AnkiRecoverySnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiRecoveryReadTarget {
    Note,
    Model,
}

/// Compact provider layout DTO. Use a sequence for Kotlin's generated list
/// codec; the authoritative native layout retains its fixed four-field array.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct AnkiReadModelLayout {
    pub name: String,
    pub field_names: Vec<String>,
    pub css: String,
    pub card_name: String,
    pub question_format: String,
    pub answer_format: String,
}

impl From<&AnkiModelLayout> for AnkiReadModelLayout {
    fn from(layout: &AnkiModelLayout) -> Self {
        Self {
            name: layout.name.clone(),
            field_names: layout.field_names.to_vec(),
            css: layout.css.clone(),
            card_name: layout.card_name.clone(),
            question_format: layout.question_format.clone(),
            answer_format: layout.answer_format.clone(),
        }
    }
}

/// Read-only platform protocol. No journal IDs, handles or mutation payloads
/// cross this boundary. Crux retains the owning handle in its response closure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiRecoveryReadOperation {
    Note {
        surface: String,
    },
    Model {
        known_id: Option<i64>,
        layout: Box<AnkiReadModelLayout>,
    },
}

impl Operation for AnkiRecoveryReadOperation {
    type Output = AnkiRecoveryReadResult;
}

/// Model(Some) means one unambiguous, complete layout match. Empty, partial or
/// failed observations cannot establish absence or authorize another mutation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiRecoveryReadResult {
    Notes(Vec<i64>),
    Model(Option<i64>),
    Failed(AnkiExportError),
}

#[derive(Debug)]
struct ReadContext {
    expected: Box<AnkiRecoverySnapshot>,
    target: AnkiRecoveryReadTarget,
}

/// Capture this handle BEFORE issuing a read and return that same handle with
/// its observation. Equal journal contents in another app/process/read attempt
/// cannot reconstruct it. The frozen pair is comparison data only.
#[derive(Clone, Debug)]
pub struct AnkiRecoveryReadRequest(Arc<ReadContext>);

impl PartialEq for AnkiRecoveryReadRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for AnkiRecoveryReadRequest {}

impl AnkiRecoveryReadRequest {
    pub(crate) fn new(
        expected: Box<AnkiRecoverySnapshot>,
        target: AnkiRecoveryReadTarget,
    ) -> Option<Self> {
        match target {
            AnkiRecoveryReadTarget::Note => expected.note.as_ref().map(|_| ())?,
            AnkiRecoveryReadTarget::Model => expected.model.as_ref().map(|_| ())?,
        }
        Some(Self(Arc::new(ReadContext { expected, target })))
    }

    pub fn expected(&self) -> &AnkiRecoverySnapshot {
        &self.0.expected
    }

    pub fn target(&self) -> AnkiRecoveryReadTarget {
        self.0.target
    }

    pub fn operation(&self) -> AnkiRecoveryReadOperation {
        match self.target() {
            AnkiRecoveryReadTarget::Note => AnkiRecoveryReadOperation::Note {
                surface: self
                    .expected()
                    .note
                    .as_ref()
                    .expect("captured note")
                    .submission
                    .note()
                    .surface()
                    .as_str()
                    .to_owned(),
            },
            AnkiRecoveryReadTarget::Model => {
                let recovery = self.expected().model.as_ref().expect("captured model");
                AnkiRecoveryReadOperation::Model {
                    known_id: recovery.known_model_id.map(i64::from),
                    layout: Box::new(recovery.creation.layout().into()),
                }
            }
        }
    }
}
