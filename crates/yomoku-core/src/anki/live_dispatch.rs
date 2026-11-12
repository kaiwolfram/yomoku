//! Typed provider transport. Only a consumed native invocation creates an owning
//! response closure; payloads and response IDs cannot recreate that owner.

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use super::{
    AnkiExistingNotes, AnkiLiveCallback, AnkiLiveIntent, AnkiLiveInvocation, AnkiLiveOutcome,
    AnkiModelCreationOutcome, AnkiReadModelLayout, AnkiSubmissionOutcome,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiLiveOperation {
    InsertNote {
        surface: String,
        reading: String,
        fields: Vec<String>,
        deck_id: i64,
        model_id: i64,
        layout: Box<AnkiReadModelLayout>,
    },
    CreateModel {
        surface: String,
        reading: String,
        deck_id: i64,
        layout: Box<AnkiReadModelLayout>,
    },
}

impl Operation for AnkiLiveOperation {
    type Output = AnkiLiveResult;
}

/// NoSubmission means positive knowledge that no mutating call was submitted.
/// Exceptions outside the adapter and malformed/wrong-kind responses are uncertain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum AnkiLiveResult {
    NoSubmission,
    NoteAdded(i64),
    NoteExisting(Vec<i64>),
    NoteUncertain(Option<i64>),
    ModelCreated(i64),
    ModelUncertain(Option<i64>),
}

impl AnkiLiveInvocation {
    /// Creation also rechecks the frozen export's surface and deck immediately
    /// before submission. Neither value is recovered from a journal ID.
    pub(crate) fn operation(
        &self,
        surface: String,
        reading: String,
        deck_id: i64,
    ) -> AnkiLiveOperation {
        match self.intent() {
            AnkiLiveIntent::Note(note) => AnkiLiveOperation::InsertNote {
                surface: note.note().surface().as_str().into(),
                reading: note.note().reading().into(),
                fields: note.note().fields().to_vec(),
                deck_id: note.deck().into(),
                model_id: note.model().into(),
                layout: Box::new(note.layout().into()),
            },
            AnkiLiveIntent::Model(model) => AnkiLiveOperation::CreateModel {
                surface,
                reading,
                deck_id,
                layout: Box::new(model.layout().into()),
            },
        }
    }

    pub(crate) fn complete_provider(self, result: AnkiLiveResult) -> AnkiLiveCallback {
        let outcome = match self.intent() {
            AnkiLiveIntent::Note(_) => {
                let uncertain = AnkiSubmissionOutcome::Uncertain {
                    known_note_id: None,
                };
                AnkiLiveOutcome::Note(match result {
                    AnkiLiveResult::NoSubmission => AnkiSubmissionOutcome::NotInserted,
                    AnkiLiveResult::NoteAdded(id) => id
                        .try_into()
                        .map(AnkiSubmissionOutcome::Added)
                        .unwrap_or(uncertain),
                    AnkiLiveResult::NoteExisting(ids) => ids
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()
                        .and_then(|ids| AnkiExistingNotes::new(ids).ok())
                        .map(AnkiSubmissionOutcome::Existing)
                        .unwrap_or(uncertain),
                    AnkiLiveResult::NoteUncertain(id) => AnkiSubmissionOutcome::Uncertain {
                        known_note_id: id.and_then(|id| id.try_into().ok()),
                    },
                    _ => uncertain,
                })
            }
            AnkiLiveIntent::Model(_) => {
                let uncertain = AnkiModelCreationOutcome::Uncertain {
                    known_model_id: None,
                };
                AnkiLiveOutcome::Model(match result {
                    AnkiLiveResult::NoSubmission => AnkiModelCreationOutcome::NotCreated,
                    AnkiLiveResult::ModelCreated(id) => id
                        .try_into()
                        .map(AnkiModelCreationOutcome::Created)
                        .unwrap_or(uncertain),
                    AnkiLiveResult::ModelUncertain(id) => AnkiModelCreationOutcome::Uncertain {
                        known_model_id: id.and_then(|id| id.try_into().ok()),
                    },
                    _ => uncertain,
                })
            }
        };
        self.complete(outcome)
    }
}
