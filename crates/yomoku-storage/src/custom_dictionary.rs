//! User-owned exact-surface fallbacks. Every operation validates the original
//! selected token and checks the separate read-only dictionary before touching
//! custom entries; caller-supplied base forms cannot bypass built-in matches.

use std::{error::Error, fmt};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{dictionary::CustomDictionaryEntry, reader::ReaderTokenKey, vocabulary::Surface};

use crate::{
    dictionary::Dictionary,
    reader_words::{selected_token_in, ReaderWordError},
    Storage, StorageError,
};

#[derive(Debug)]
pub enum CustomDictionaryError {
    Selection(ReaderWordError),
    Storage(StorageError),
    BuiltInMatch,
    InvalidEntry,
}

impl fmt::Display for CustomDictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Selection(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
            Self::BuiltInMatch => f.write_str("a built-in dictionary match already exists"),
            Self::InvalidEntry => f.write_str("invalid custom dictionary entry"),
        }
    }
}

impl Error for CustomDictionaryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Selection(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::BuiltInMatch | Self::InvalidEntry => None,
        }
    }
}

impl From<ReaderWordError> for CustomDictionaryError {
    fn from(value: ReaderWordError) -> Self {
        Self::Selection(value)
    }
}

impl From<rusqlite::Error> for CustomDictionaryError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Storage(value.into())
    }
}

impl Storage {
    /// Read-only fallback lookup; None means no saved entry or a deleted document.
    /// Built-in matches and failed dictionary reads are distinct errors, never
    /// permission to display a custom override. No registration or history writes.
    pub fn load_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: ReaderTokenKey,
        token_index: u64,
    ) -> Result<Option<CustomDictionaryEntry>, CustomDictionaryError> {
        let transaction = self.connection.transaction()?;
        let Some(surface) = missing_surface_in(&transaction, dictionary, key, token_index)? else {
            return Ok(None);
        };
        let entry = read_entry_in(&transaction, surface)?;
        transaction.commit()?;
        Ok(entry)
    }

    /// Create or replace one exact spelling's meaning/reading atomically. The
    /// selected original Mode B token supplies surface/base identity. Both fields
    /// must be nonblank; literal Unicode is preserved. None means deleted document.
    /// Does not edit analyzer metadata, built-ins, vocabulary, time or encounters.
    pub fn save_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: ReaderTokenKey,
        token_index: u64,
        english_meaning: String,
        reading: String,
    ) -> Result<Option<CustomDictionaryEntry>, CustomDictionaryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(surface) = missing_surface_in(&transaction, dictionary, key, token_index)? else {
            return Ok(None);
        };
        let entry = CustomDictionaryEntry::new(surface, english_meaning, reading)
            .map_err(|_| CustomDictionaryError::InvalidEntry)?;
        // Validate saved values before replacement; corrupt state is never repaired
        // implicitly. Identical retries are write-free, even with update triggers.
        if read_entry_in(&transaction, entry.surface().clone())?.as_ref() != Some(&entry) {
            transaction.execute(
                "INSERT INTO custom_dictionary_entries
                 (surface, semantics_version, english_meaning, reading) VALUES (?1, 1, ?2, ?3)
                 ON CONFLICT(surface) DO UPDATE SET
                 english_meaning = excluded.english_meaning, reading = excluded.reading",
                (
                    entry.surface().as_str(),
                    entry.english_meaning(),
                    entry.reading(),
                ),
            )?;
        }
        transaction.commit()?;
        Ok(Some(entry))
    }

    /// Remove only the selected spelling's custom entry. Repeating deletion or
    /// selecting a deleted document returns false without registering anything.
    pub fn delete_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: ReaderTokenKey,
        token_index: u64,
    ) -> Result<bool, CustomDictionaryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(surface) = missing_surface_in(&transaction, dictionary, key, token_index)? else {
            return Ok(false);
        };
        let exists = read_entry_in(&transaction, surface.clone())?.is_some();
        if exists {
            transaction.execute(
                "DELETE FROM custom_dictionary_entries WHERE surface = ?1",
                [surface.as_str()],
            )?;
        }
        transaction.commit()?;
        Ok(exists)
    }
}

fn missing_surface_in(
    connection: &Connection,
    dictionary: &Dictionary,
    key: ReaderTokenKey,
    token_index: u64,
) -> Result<Option<Surface>, CustomDictionaryError> {
    let Some(token) = selected_token_in(connection, key, token_index)? else {
        return Ok(None);
    };
    let candidates = dictionary
        .lookup(&token.surface, token.dictionary_form.as_deref())
        .map_err(ReaderWordError::from)?;
    if !candidates.words.is_empty() || !candidates.names.is_empty() {
        return Err(CustomDictionaryError::BuiltInMatch);
    }
    Surface::new(token.surface)
        .map(Some)
        .map_err(|_| CustomDictionaryError::InvalidEntry)
}

fn read_entry_in(
    connection: &Connection,
    surface: Surface,
) -> Result<Option<CustomDictionaryEntry>, CustomDictionaryError> {
    let row = connection
        .query_row(
            "SELECT semantics_version, english_meaning, reading
             FROM custom_dictionary_entries WHERE surface = ?1",
            [surface.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    row.map(|(version, meaning, reading)| {
        if version != 1 {
            return Err(CustomDictionaryError::InvalidEntry);
        }
        CustomDictionaryEntry::new(surface, meaning, reading)
            .map_err(|_| CustomDictionaryError::InvalidEntry)
    })
    .transpose()
}
