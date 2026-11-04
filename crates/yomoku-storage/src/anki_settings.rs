use rusqlite::{OptionalExtension, TransactionBehavior};
use yomoku_core::anki::AnkiDeckId;

use crate::{Storage, StorageError};

impl Storage {
    /// The configured preference only. Availability, permission and a nonfiltered
    /// deck must be checked against the current provider before export.
    pub fn anki_default_deck(&self) -> Result<Option<AnkiDeckId>, StorageError> {
        read_default_deck(&self.connection)
    }

    /// Publish only after commit. Changing/clearing the preference cannot retarget
    /// in-memory frozen exports.
    pub fn save_anki_default_deck(
        &mut self,
        deck: Option<AnkiDeckId>,
    ) -> Result<Option<AnkiDeckId>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Missing/corrupt rows fail closed rather than being silently repaired.
        read_default_deck(&transaction)?;
        let updated = transaction.execute(
            "UPDATE anki_settings SET default_deck_id = ?1 WHERE id = 1",
            [deck.map(i64::from)],
        )?;
        if updated != 1 || read_default_deck(&transaction)? != deck {
            return Err(StorageError::InvalidSettings);
        }
        transaction.commit()?;
        Ok(deck)
    }
}

fn read_default_deck(
    connection: &rusqlite::Connection,
) -> Result<Option<AnkiDeckId>, StorageError> {
    let stored: Option<Option<i64>> = connection
        .query_row(
            "SELECT default_deck_id FROM anki_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    stored
        .ok_or(StorageError::InvalidSettings)?
        .map(|value| AnkiDeckId::try_from(value).map_err(|_| StorageError::InvalidSettings))
        .transpose()
}
