//! Both external journals must be restored in one coherent SQLite snapshot.

use yomoku_core::anki::AnkiRecoverySnapshot;

use crate::{anki::AnkiJournalError, Storage};

impl Storage {
    /// Read-only recovery, never live callback tokens. A transaction keeps a
    /// concurrent writer from making two individually empty reads span a live
    /// note/model obligation. Any read/commit error fails the whole restore.
    pub fn anki_recovery_snapshot(&mut self) -> Result<AnkiRecoverySnapshot, AnkiJournalError> {
        let transaction = self.connection.transaction()?;
        let snapshot = AnkiRecoverySnapshot {
            note: crate::anki::read_recovery(&transaction)?,
            model: crate::anki_model::read_recovery(&transaction)?,
        };
        transaction.commit()?;
        Ok(snapshot)
    }
}
