//! Own one native read handle, then commit only its captured journal evidence.

use super::*;
use crate::anki::{
    AnkiExistingNotes, AnkiExportError, AnkiModelReconciliation, AnkiReconciliation,
    AnkiRecoveryCommit, AnkiRecoveryObservation, AnkiRecoveryReadRequest, AnkiRecoveryReadResult,
    AnkiRecoveryReadTarget,
};

impl Model {
    /// Native diagnostics only. The shell receives the read operation and Crux
    /// keeps this handle in its response closure; it never polls this accessor.
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
                    model.anki_recovery_read = Some(request.clone());
                    model.anki_recovery_read_error = None;
                    return Some(Command::request_from_shell(request.operation()).then_send(
                        move |result| {
                            Event::AnkiRecoveryProviderFinished(Box::new(request), result)
                        },
                    ));
                }
                Some(Command::done())
            }
            Event::AnkiRecoveryProviderFinished(request, result) => {
                if model.anki_recovery_read.as_ref() != Some(request.as_ref()) {
                    return Some(Command::done());
                }
                let observation = match provider_observation(request, result) {
                    Ok(observation) => observation,
                    Err(error) => {
                        model.anki_recovery_read = None;
                        model.anki_recovery_read_error = Some(error);
                        return Some(Command::done());
                    }
                };
                let command = self.update_anki_recovery_read(
                    &Event::AnkiRecoveryReadFinished(request.clone(), Box::new(observation)),
                    model,
                );
                if model.anki_recovery_read_error.is_none() {
                    if let AnkiRecoveryReadResult::Failed(error) = result {
                        model.anki_recovery_read_error = Some(*error);
                    }
                }
                command
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

fn provider_observation(
    request: &AnkiRecoveryReadRequest,
    result: &AnkiRecoveryReadResult,
) -> Result<AnkiRecoveryObservation, AnkiExportError> {
    Ok(match (request.target(), result) {
        (AnkiRecoveryReadTarget::Note, AnkiRecoveryReadResult::Notes(ids)) => {
            let ids = ids
                .iter()
                .copied()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| AnkiExportError::InvalidResponse)?;
            AnkiRecoveryObservation::Note(if ids.is_empty() {
                AnkiReconciliation::Unresolved
            } else {
                AnkiReconciliation::Existing(
                    AnkiExistingNotes::new(ids).map_err(|_| AnkiExportError::InvalidResponse)?,
                )
            })
        }
        (AnkiRecoveryReadTarget::Model, AnkiRecoveryReadResult::Model(id)) => {
            let recovery = request.expected().model.as_ref().expect("captured model");
            AnkiRecoveryObservation::Model(match id {
                Some(id) => AnkiModelReconciliation::Observed {
                    id: (*id)
                        .try_into()
                        .map_err(|_| AnkiExportError::InvalidResponse)?,
                    layout: Box::new(recovery.creation.layout().clone()),
                },
                None => AnkiModelReconciliation::Unresolved,
            })
        }
        (AnkiRecoveryReadTarget::Note, AnkiRecoveryReadResult::Failed(error)) => {
            AnkiRecoveryObservation::Note(AnkiReconciliation::Failed(*error))
        }
        (AnkiRecoveryReadTarget::Model, AnkiRecoveryReadResult::Failed(_)) => {
            AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved)
        }
        _ => return Err(AnkiExportError::InvalidResponse),
    })
}

#[cfg(test)]
mod tests;
