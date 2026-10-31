//! Own one native read handle, then commit only its captured journal evidence.

use super::*;
use crate::anki::{
    AnkiExportError, AnkiRecoveryCommit, AnkiRecoveryObservation, AnkiRecoveryReadRequest,
    AnkiRecoveryReadTarget,
};

impl Model {
    /// A future provider/FFI dispatcher must capture this exact native handle
    /// before reading. No shell polling, provider call or mutation is wired yet.
    pub fn pending_anki_recovery_read(&self) -> Option<&AnkiRecoveryReadRequest> {
        self.anki_recovery_read.as_ref()
    }

    pub fn anki_recovery_read_error(&self) -> Option<AnkiExportError> {
        self.anki_recovery_read_error
    }
}

impl Yomoku {
    pub(super) fn update_anki_recovery_read(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::BeginAnkiRecoveryRead(target) => {
                let AnkiRecoveryState::Restored(snapshot) = &model.anki_recovery else {
                    return Some(Command::done());
                };
                if model.anki_recovery_read.is_some()
                    || model.anki_recovery_commit.is_some()
                    || model.anki_recovery_commit_error.is_some()
                {
                    return Some(Command::done());
                }
                if let Some(request) = AnkiRecoveryReadRequest::new(snapshot.clone(), *target) {
                    model.anki_recovery_read = Some(request);
                    model.anki_recovery_read_error = None;
                }
                Some(Command::done())
            }
            Event::AnkiRecoveryReadFinished(request, observation) => {
                if model.anki_recovery_read.as_ref() != Some(request.as_ref()) {
                    return Some(Command::done());
                }
                model.anki_recovery_read = None;
                let valid_target = matches!(
                    (request.target(), observation.as_ref()),
                    (
                        AnkiRecoveryReadTarget::Note,
                        AnkiRecoveryObservation::Note(_)
                    ) | (
                        AnkiRecoveryReadTarget::Model,
                        AnkiRecoveryObservation::Model(_)
                    )
                );
                if !valid_target
                    || !matches!(&model.anki_recovery,
                        AnkiRecoveryState::Restored(snapshot) if snapshot.as_ref() == request.expected())
                    || model.anki_recovery_commit.is_some()
                    || model.anki_recovery_commit_error.is_some()
                {
                    model.anki_recovery_read_error = Some(AnkiExportError::InvalidResponse);
                    return Some(Command::done());
                }
                // The provider cannot supply a replacement snapshot. Keep the
                // gate closed until the existing durable commit acknowledges it.
                let command = Event::CommitAnkiRecovery(Box::new(AnkiRecoveryCommit {
                    expected: Box::new(request.expected().clone()),
                    observation: observation.as_ref().clone(),
                }));
                self.update_anki_reconciliation(&command, model)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
