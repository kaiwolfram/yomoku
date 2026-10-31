//! Native recovery data cannot authorize or reconstruct a live provider call.

use super::{
    AnkiModelCreation, AnkiModelId, AnkiModelLayout, AnkiNoteId, AnkiReconciliation, AnkiSubmission,
};
use crate::{document::InvalidTextValue, storage::DocumentError};

macro_rules! journal_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $name(i64);

        impl $name {
            pub fn value(self) -> i64 {
                self.0
            }
        }

        impl TryFrom<i64> for $name {
            type Error = InvalidTextValue;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                (value > 0).then_some(Self(value)).ok_or(InvalidTextValue)
            }
        }
    };
}

journal_id!(AnkiSubmissionId);
journal_id!(AnkiModelCreationId);

/// Pending and uncertain are indistinguishable; only read-only reconciliation
/// may use restored records. Existence never proves requested deck placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiRecovery {
    pub id: AnkiSubmissionId,
    pub submission: AnkiSubmission,
    pub known_note_id: Option<AnkiNoteId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiModelRecovery {
    pub id: AnkiModelCreationId,
    pub creation: AnkiModelCreation,
    pub known_model_id: Option<AnkiModelId>,
}

/// Both journals from one database read snapshot. A partial read is not a result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnkiRecoverySnapshot {
    pub note: Option<AnkiRecovery>,
    pub model: Option<AnkiModelRecovery>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AnkiRecoveryState {
    #[default]
    Unrestored,
    Loading,
    Failed(DocumentError),
    Restored(Box<AnkiRecoverySnapshot>),
}

impl AnkiRecoveryState {
    /// This satisfies ONLY the startup prerequisite. A future fresh mutation
    /// still requires current-selection validation and a committed journal token.
    pub fn allows_preparation(&self) -> bool {
        matches!(self, Self::Restored(snapshot) if snapshot.note.is_none() && snapshot.model.is_none())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiModelReconciliation {
    /// Full normal/sort-zero/single-card provider readback, not name-only
    /// discovery. A matching layout establishes usability, never ownership.
    Observed {
        id: AnkiModelId,
        layout: Box<AnkiModelLayout>,
    },
    /// Empty, failed, partial, ambiguous or incompatible provider reads.
    Unresolved,
}

impl AnkiModelReconciliation {
    pub fn resolves(&self, recovery: &AnkiModelRecovery) -> bool {
        matches!(self, Self::Observed { id, layout }
            if layout.as_ref() == recovery.creation.layout()
                && recovery.known_model_id.is_none_or(|known| known == *id))
    }
}

/// Evidence from a read-only provider observation, never a mutation outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiRecoveryObservation {
    Note(AnkiReconciliation),
    Model(AnkiModelReconciliation),
}

/// Capture the exact restored pair at read dispatch. A changed obligation must
/// be restored and observed again, never reinterpreted using a newer snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiRecoveryCommit {
    pub expected: Box<AnkiRecoverySnapshot>,
    pub observation: AnkiRecoveryObservation,
}

impl AnkiRecoveryCommit {
    /// Validate the targeted obligation and predict only the unresolved slots.
    /// Pending -> uncertain keeps identical recovery data. Note existence does
    /// not establish placement; model evidence must match full layout/known ID.
    pub fn expected_result(&self) -> Option<Box<AnkiRecoverySnapshot>> {
        let mut result = self.expected.clone();
        match &self.observation {
            AnkiRecoveryObservation::Note(observation) => {
                result.note.as_ref()?;
                if matches!(observation, AnkiReconciliation::Existing(_)) {
                    result.note = None;
                }
            }
            AnkiRecoveryObservation::Model(observation) => {
                if observation.resolves(result.model.as_ref()?) {
                    result.model = None;
                }
            }
        }
        Some(result)
    }
}
