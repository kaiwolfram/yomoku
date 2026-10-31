//! Native read handles retain provenance, never permission to mutate a provider.

use std::sync::Arc;

use super::AnkiRecoverySnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnkiRecoveryReadTarget {
    Note,
    Model,
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
}
