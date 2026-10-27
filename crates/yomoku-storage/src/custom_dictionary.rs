//! User-owned exact-surface fallbacks. Every operation validates the current
//! selected token and checks the separate read-only dictionary before touching
//! custom entries; caller-supplied base forms cannot bypass built-in matches.

use std::{error::Error, fmt};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    dictionary::CustomDictionaryEntry, effective_reader::EffectiveReaderTokenKey,
    reader::ReaderTokenKey, vocabulary::Surface,
};

use crate::{
    dictionary::Dictionary,
    reader_words::{selected_effective_token_in, selected_token_in, ReaderWordError},
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
        let entry = save_entry_in(&transaction, surface, english_meaning, reading)?;
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
        let exists = delete_entry_in(&transaction, surface)?;
        transaction.commit()?;
        Ok(exists)
    }

    /// Read an effective segment's global custom fallback. Re-resolves the full
    /// frozen selection in the same read transaction; stale rules/metadata fail.
    /// Exact metadata alone may supply a base; unresolved manual spans use only
    /// their surface. None means no entry or a deleted document. No registration.
    pub fn load_effective_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
    ) -> Result<Option<CustomDictionaryEntry>, CustomDictionaryError> {
        let transaction = self.connection.transaction()?;
        let Some(surface) =
            missing_effective_surface_in(&transaction, dictionary, key, segment_index)?
        else {
            return Ok(None);
        };
        let entry = read_entry_in(&transaction, surface)?;
        transaction.commit()?;
        Ok(entry)
    }

    /// Create/replace the selected effective spelling's meaning/reading. An
    /// immediate transaction locks resolution validation, built-in protection and
    /// mutation together. Never registers vocabulary or invents analyzer metadata.
    /// Literal fields, corrupt-entry rejection and exact retries match original B.
    pub fn save_effective_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
        english_meaning: String,
        reading: String,
    ) -> Result<Option<CustomDictionaryEntry>, CustomDictionaryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(surface) =
            missing_effective_surface_in(&transaction, dictionary, key, segment_index)?
        else {
            return Ok(None);
        };
        let entry = save_entry_in(&transaction, surface, english_meaning, reading)?;
        transaction.commit()?;
        Ok(Some(entry))
    }

    /// Remove only the current effective spelling's fallback. Built-in matches,
    /// stale keys and corrupt saved entries fail before deletion. Missing entries
    /// and deleted documents return false without recreating vocabulary.
    pub fn delete_effective_reader_custom_entry(
        &mut self,
        dictionary: &Dictionary,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
    ) -> Result<bool, CustomDictionaryError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(surface) =
            missing_effective_surface_in(&transaction, dictionary, key, segment_index)?
        else {
            return Ok(false);
        };
        let exists = delete_entry_in(&transaction, surface)?;
        transaction.commit()?;
        Ok(exists)
    }
}

fn save_entry_in(
    connection: &Connection,
    surface: Surface,
    english_meaning: String,
    reading: String,
) -> Result<CustomDictionaryEntry, CustomDictionaryError> {
    let entry = CustomDictionaryEntry::new(surface, english_meaning, reading)
        .map_err(|_| CustomDictionaryError::InvalidEntry)?;
    // Validate saved values before replacement; corrupt state is never repaired
    // implicitly. Identical retries are write-free, even with update triggers.
    if read_entry_in(connection, entry.surface().clone())?.as_ref() != Some(&entry) {
        connection.execute(
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
    Ok(entry)
}

fn delete_entry_in(
    connection: &Connection,
    surface: Surface,
) -> Result<bool, CustomDictionaryError> {
    let exists = read_entry_in(connection, surface.clone())?.is_some();
    if exists {
        connection.execute(
            "DELETE FROM custom_dictionary_entries WHERE surface = ?1",
            [surface.as_str()],
        )?;
    }
    Ok(exists)
}

fn missing_effective_surface_in(
    connection: &Connection,
    dictionary: &Dictionary,
    key: &EffectiveReaderTokenKey,
    segment_index: u64,
) -> Result<Option<Surface>, CustomDictionaryError> {
    let Some(token) = selected_effective_token_in(connection, key, segment_index)? else {
        return Ok(None);
    };
    missing_surface(
        dictionary,
        token.surface,
        token
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.dictionary_form.as_ref().map(|form| form.as_str())),
    )
    .map(Some)
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
    missing_surface(dictionary, token.surface, token.dictionary_form.as_deref()).map(Some)
}

fn missing_surface(
    dictionary: &Dictionary,
    surface: String,
    dictionary_form: Option<&str>,
) -> Result<Surface, CustomDictionaryError> {
    let candidates = dictionary
        .lookup(&surface, dictionary_form)
        .map_err(ReaderWordError::from)?;
    if !candidates.words.is_empty() || !candidates.names.is_empty() {
        return Err(CustomDictionaryError::BuiltInMatch);
    }
    Surface::new(surface).map_err(|_| CustomDictionaryError::InvalidEntry)
}

pub(crate) fn read_entry_in(
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
