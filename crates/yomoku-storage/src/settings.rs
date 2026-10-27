use rusqlite::{OptionalExtension, TransactionBehavior};
use yomoku_core::pagination::PageLength;
use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};

use crate::{Storage, StorageError};

impl Storage {
    pub fn reader_typography(&self) -> Result<ReaderTypography, StorageError> {
        read_reader_typography(&self.connection)
    }

    /// Persist both visual preferences atomically; publish only committed values.
    pub fn save_reader_typography(
        &mut self,
        typography: ReaderTypography,
    ) -> Result<ReaderTypography, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        read_page_length(&transaction)?;
        read_reader_typography(&transaction)?;
        transaction.execute(
            "UPDATE reader_settings SET font_size = ?1, line_spacing = ?2 WHERE id = 1",
            [typography.font_size.sp(), typography.line_spacing.percent()],
        )?;
        transaction.commit()?;
        Ok(typography)
    }

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

pub(crate) fn read_reader_typography(
    connection: &rusqlite::Connection,
) -> Result<ReaderTypography, StorageError> {
    let values: Option<(i64, i64)> = connection
        .query_row(
            "SELECT font_size, line_spacing FROM reader_settings WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (size, spacing) = values.ok_or(StorageError::InvalidSettings)?;
    Ok(ReaderTypography {
        font_size: ReaderFontSize::ALL
            .into_iter()
            .find(|value| value.sp() == size)
            .ok_or(StorageError::InvalidSettings)?,
        line_spacing: ReaderLineSpacing::ALL
            .into_iter()
            .find(|value| value.percent() == spacing)
            .ok_or(StorageError::InvalidSettings)?,
    })
}

pub(crate) fn read_page_length(
    connection: &rusqlite::Connection,
) -> Result<PageLength, StorageError> {
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
