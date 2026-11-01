//! Native live ownership. Restored payloads and journal IDs cannot mint requests
//! or invocation permits, and none of these ownership types has a shell codec.

use std::sync::{Arc, Weak};

use super::{
    AnkiExistingNotes, AnkiModelCreation, AnkiModelId, AnkiNoteId, AnkiRecoverySnapshot,
    AnkiSubmission,
};
use crate::storage::DocumentError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiLiveIntent {
    Note(Box<AnkiSubmission>),
    Model(Box<AnkiModelCreation>),
}

/// NotInserted requires positive knowledge that no note mutation was submitted.
/// Every failure after submission, including placement/readback, is uncertain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiSubmissionOutcome {
    NotInserted,
    Added(AnkiNoteId),
    Existing(AnkiExistingNotes),
    Uncertain { known_note_id: Option<AnkiNoteId> },
}

/// NotCreated requires knowledge that no model mutation was submitted.
/// Partial creation and lost insert/update responses are uncertain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiModelCreationOutcome {
    NotCreated,
    Created(AnkiModelId),
    Uncertain { known_model_id: Option<AnkiModelId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnkiLiveOutcome {
    Note(AnkiSubmissionOutcome),
    Model(AnkiModelCreationOutcome),
}

#[derive(Clone, Debug)]
pub struct AnkiLiveRequest(Arc<AnkiLiveIntent>);

impl PartialEq for AnkiLiveRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AnkiLiveRequest {}

impl AnkiLiveRequest {
    pub(crate) fn new(intent: AnkiLiveIntent) -> Self {
        Self(Arc::new(intent))
    }

    pub fn intent(&self) -> &AnkiLiveIntent {
        &self.0
    }

    /// Non-owning identity for a worker's consumed-request ledger. It retains
    /// neither frozen note fields nor abandoned session payloads.
    pub fn downgrade(&self) -> AnkiLiveRequestWeak {
        AnkiLiveRequestWeak(Arc::downgrade(&self.0))
    }

    pub(crate) fn accepts_begin(&self, snapshot: &AnkiRecoverySnapshot) -> bool {
        match (self.intent(), &snapshot.note, &snapshot.model) {
            (AnkiLiveIntent::Note(payload), Some(note), None) => {
                &note.submission == payload.as_ref() && note.known_note_id.is_none()
            }
            (AnkiLiveIntent::Model(payload), None, Some(model)) => {
                &model.creation == payload.as_ref() && model.known_model_id.is_none()
            }
            _ => false,
        }
    }
}

pub struct AnkiLiveRequestWeak(Weak<AnkiLiveIntent>);

impl AnkiLiveRequestWeak {
    pub fn is_live(&self) -> bool {
        self.0.strong_count() != 0
    }

    pub fn matches(&self, request: &AnkiLiveRequest) -> bool {
        self.0.ptr_eq(&Arc::downgrade(&request.0))
    }
}

/// Move-only native permit. Taking it consumes the app's Ready state. Typed
/// dispatch retains it in the response closure; the provider refreshes all
/// prerequisites and returns an outcome even when no mutation was submitted.
#[derive(Debug)]
pub struct AnkiLiveInvocation {
    request: AnkiLiveRequest,
}

impl AnkiLiveInvocation {
    pub(crate) fn new(request: AnkiLiveRequest) -> Self {
        Self { request }
    }

    pub fn intent(&self) -> &AnkiLiveIntent {
        self.request.intent()
    }

    pub fn complete(self, outcome: AnkiLiveOutcome) -> AnkiLiveCallback {
        AnkiLiveCallback {
            request: self.request,
            outcome,
        }
    }
}

/// Only a consumed invocation permit can produce this callback. Cloned or late
/// callback delivery is harmless: both app and worker check and consume owners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnkiLiveCallback {
    request: AnkiLiveRequest,
    outcome: AnkiLiveOutcome,
}

impl AnkiLiveCallback {
    pub fn request(&self) -> &AnkiLiveRequest {
        &self.request
    }

    pub fn outcome(&self) -> &AnkiLiveOutcome {
        &self.outcome
    }

    pub fn expected_result(
        &self,
        before: &AnkiRecoverySnapshot,
    ) -> Option<Box<AnkiRecoverySnapshot>> {
        if !self.request.accepts_begin(before) {
            return None;
        }
        let mut after = Box::new(before.clone());
        match (self.request.intent(), &self.outcome) {
            (AnkiLiveIntent::Note(_), AnkiLiveOutcome::Note(outcome)) => match outcome {
                AnkiSubmissionOutcome::Uncertain { known_note_id } => {
                    after.note.as_mut()?.known_note_id = *known_note_id;
                }
                _ => after.note = None,
            },
            (AnkiLiveIntent::Model(_), AnkiLiveOutcome::Model(outcome)) => match outcome {
                AnkiModelCreationOutcome::Uncertain { known_model_id } => {
                    after.model.as_mut()?.known_model_id = *known_model_id;
                }
                _ => after.model = None,
            },
            _ => return None,
        }
        Some(after)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnkiLiveProgress {
    Beginning,
    Ready,
    Invoking,
    Saving,
    Finished,
    /// Requires explicit read-only restoration, never another invocation.
    Failed(DocumentError),
}

pub(crate) struct AnkiLiveState {
    pub request: AnkiLiveRequest,
    pub progress: AnkiLiveProgress,
    pub begun: Option<Box<AnkiRecoverySnapshot>>,
    pub callback: Option<AnkiLiveCallback>,
}
