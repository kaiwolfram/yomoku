//! Both external journals must be restored in one coherent SQLite snapshot.

use yomoku_core::anki::AnkiRecoverySnapshot;

use crate::{anki::AnkiJournalError, Storage};

impl Storage {
    /// Read-only recovery, never live callback tokens. A transaction keeps a
    /// concurrent writer from making two individually empty reads span a live
    /// note/model obligation. Any read/commit error fails the whole restore.
    pub fn anki_recovery_snapshot(&mut self) -> Result<AnkiRecoverySnapshot, AnkiJournalError> {
        let transaction = self.connection.transaction()?;
        let snapshot = read_snapshot(&transaction)?;
        transaction.commit()?;
        Ok(snapshot)
    }
}

impl Storage {
    /// Commit only evidence for the exact frozen pair observed by the caller.
    /// Recheck under the same write transaction as both reconciliation and the
    /// resulting gate snapshot. A failed or lost commit response requires a
    /// fresh read-only restore, not replay of this command or provider writes.
    pub fn commit_anki_recovery(
        &mut self,
        command: &yomoku_core::anki::AnkiRecoveryCommit,
    ) -> Result<Box<AnkiRecoverySnapshot>, AnkiJournalError> {
        use rusqlite::TransactionBehavior;
        use yomoku_core::anki::AnkiRecoveryObservation;

        let expected_result = command
            .expected_result()
            .ok_or(AnkiJournalError::StaleCallback)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if read_snapshot(&transaction)? != *command.expected {
            return Err(AnkiJournalError::StaleCallback);
        }
        match &command.observation {
            AnkiRecoveryObservation::Note(observation) => crate::anki::reconcile_in(
                &transaction,
                command
                    .expected
                    .note
                    .as_ref()
                    .ok_or(AnkiJournalError::StaleCallback)?
                    .id,
                observation.clone(),
            )?,
            AnkiRecoveryObservation::Model(observation) => crate::anki_model::reconcile_in(
                &transaction,
                command
                    .expected
                    .model
                    .as_ref()
                    .ok_or(AnkiJournalError::StaleCallback)?
                    .id,
                observation.clone(),
            )?,
        }
        let result = Box::new(read_snapshot(&transaction)?);
        if result != expected_result {
            return Err(AnkiJournalError::InvalidRecord);
        }
        transaction.commit()?;
        Ok(result)
    }
}

fn read_snapshot(
    connection: &rusqlite::Connection,
) -> Result<AnkiRecoverySnapshot, AnkiJournalError> {
    Ok(AnkiRecoverySnapshot {
        note: crate::anki::read_recovery(connection)?,
        model: crate::anki_model::read_recovery(connection)?,
    })
}
