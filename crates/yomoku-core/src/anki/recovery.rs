//! Native recovery data cannot authorize or reconstruct a live provider call.

use super::{AnkiModelCreation, AnkiModelId, AnkiNoteId, AnkiSubmission};
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
