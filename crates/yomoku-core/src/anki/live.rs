//! In-memory native ownership. Only the active owner may consume an invocation;
//! these ownership types have no shell codec.

use std::sync::Arc;

use super::{AnkiExistingNotes, AnkiModelCreation, AnkiModelId, AnkiNoteId, AnkiSubmission};

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnkiLiveProgress {
    Ready,
    Invoking,
    Finished,
}

pub(crate) struct AnkiLiveState {
    pub request: AnkiLiveRequest,
    pub progress: AnkiLiveProgress,
    pub callback: Option<AnkiLiveCallback>,
}
