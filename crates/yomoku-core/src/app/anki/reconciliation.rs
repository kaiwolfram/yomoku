//! Commit only read-only evidence for the exact retained recovery snapshot.

use super::*;

impl Model {
    pub fn anki_recovery_saving(&self) -> bool {
        self.anki_recovery_commit.is_some()
    }

    pub fn anki_recovery_commit_error(&self) -> Option<DocumentError> {
        self.anki_recovery_commit_error
    }
}

impl Yomoku {
    pub(super) fn update_anki_reconciliation(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::CommitAnkiRecovery(command) => {
                let AnkiRecoveryState::Restored(snapshot) = &model.anki_recovery else {
                    return Some(Command::done());
                };
                if model.anki_recovery_read.is_some()
                    || model.anki_recovery_commit.is_some()
                    || model.anki_recovery_commit_error.is_some()
                    || command.expected != *snapshot
                    || command.expected_result().is_none()
                {
                    return Some(Command::done());
                }
                let Some(id) = model.anki_recovery_sequence.checked_add(1) else {
                    model.anki_recovery_commit_error = Some(DocumentError::Unavailable);
                    return Some(Command::done());
                };
                model.anki_recovery_sequence = id;
                model.anki_recovery_commit = Some((id, command.clone()));
                Some(
                    Command::request_from_shell(StorageOperation::CommitAnkiRecovery(
                        command.clone(),
                    ))
                    .then_send(move |result| Event::AnkiRecoveryCommitFinished(id, result)),
                )
            }
            Event::AnkiRecoveryCommitFinished(id, result) => {
                let Some((pending_id, command)) = &model.anki_recovery_commit else {
                    return Some(Command::done());
                };
                if pending_id != id {
                    return Some(Command::done());
                }
                match result {
                    StorageResult::AnkiRecoveryCommitted(snapshot)
                        if command.expected_result().as_ref() == Some(snapshot) =>
                    {
                        model.anki_recovery = AnkiRecoveryState::Restored(snapshot.clone());
                        model.anki_recovery_commit_error = None;
                    }
                    StorageResult::Failed(error) => model.anki_recovery_commit_error = Some(*error),
                    _ => model.anki_recovery_commit_error = Some(DocumentError::Unavailable),
                }
                // A failed/lost commit response retains the old obligation.
                // Retry restores both journals read-only instead of replaying.
                model.anki_recovery_commit = None;
                Some(Command::done())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
