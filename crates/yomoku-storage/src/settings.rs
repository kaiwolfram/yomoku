use rusqlite::{OptionalExtension, TransactionBehavior};
use yomoku_core::pagination::PageLength;

use crate::{Storage, StorageError};

impl Storage {
    pub fn page_length(&self) -> Result<PageLength, StorageError> {
        read_page_length(&self.connection)
    }

    /// Return only after commit; never replace a missing or corrupt settings row.
    pub fn save_page_length(&mut self, length: PageLength) -> Result<PageLength, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        read_page_length(&transaction)?;
        transaction.execute(
            "UPDATE reader_settings SET page_length = ?1 WHERE id = 1",
            [length.characters() as i64],
        )?;
        transaction.commit()?;
        Ok(length)
    }
}

fn read_page_length(connection: &rusqlite::Connection) -> Result<PageLength, StorageError> {
    let value: Option<i64> = connection
        .query_row(
            "SELECT page_length FROM reader_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    PageLength::ALL
        .into_iter()
        .find(|length| Some(length.characters() as i64) == value)
        .ok_or(StorageError::InvalidSettings)
}
