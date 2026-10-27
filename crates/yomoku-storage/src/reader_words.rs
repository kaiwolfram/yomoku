//! Selected-word read composition for the runtime's I/O worker. Dictionary
//! assets remain separate from mutable state; neither connection is written.

use std::{error::Error, fmt};

use rusqlite::Connection;
use yomoku_core::{
    dictionary::DictionaryCandidates,
    effective_reader::{EffectiveReaderTokenKey, EffectiveReaderWordDetails},
    reader::{ReaderCustomEntry, ReaderToken, ReaderTokenKey, ReaderVocabulary},
};

use crate::{
    dictionary::{Dictionary, DictionaryError},
    reader_tokens::{load_effective_reader_tokens_in, load_reader_tokens_in},
    Storage, StorageError,
};

const COUNT_SQL: &str = "SELECT COUNT(*) FROM reading_occurrences WHERE vocabulary_id = ?1";

pub use yomoku_core::reader::ReaderWordDetails;

#[derive(Debug)]
pub enum ReaderWordError {
    Storage(StorageError),
    Dictionary(DictionaryError),
    /// A stale base identity or an index outside this page's Japanese tokens.
    InvalidSelection,
    InvalidCustomEntry,
}

impl fmt::Display for ReaderWordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(f),
            Self::Dictionary(error) => error.fmt(f),
            Self::InvalidSelection => f.write_str("invalid or stale reader word selection"),
            Self::InvalidCustomEntry => f.write_str("invalid custom dictionary entry"),
        }
    }
}

impl Error for ReaderWordError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Dictionary(error) => Some(error),
            Self::InvalidSelection | Self::InvalidCustomEntry => None,
        }
    }
}

impl From<StorageError> for ReaderWordError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<DictionaryError> for ReaderWordError {
    fn from(value: DictionaryError) -> Self {
        Self::Dictionary(value)
    }
}

impl Storage {
    /// Resolve a selected original Mode B token, live status and lifetime count
    /// from one consistent user-state read transaction. Surface/base forms come
    /// from validated stored analysis, never from caller-supplied text. Includes
    /// crossing-page fragments under their full original token identity.
    ///
    /// None means the document was deleted. Missing/stale analysis or a token
    /// outside the supplied page is InvalidSelection. Dictionary failures fail
    /// the entire lookup and are distinguishable from successful empty matches.
    /// No analyzer loading, registration, status, position or encounter writes.
    pub fn load_reader_word(
        &mut self,
        dictionary: &Dictionary,
        key: ReaderTokenKey,
        token_index: u64,
    ) -> Result<Option<ReaderWordDetails>, ReaderWordError> {
        let transaction = self.connection.transaction().map_err(StorageError::from)?;
        let Some(token) = selected_token_in(&transaction, key, token_index)? else {
            return Ok(None);
        };
        let (encountered, candidates, custom_entry) = word_details_in(
            &transaction,
            dictionary,
            &token.surface,
            token.dictionary_form.as_deref(),
            token.vocabulary,
        )?;
        transaction.commit().map_err(StorageError::from)?;
        Ok(Some(ReaderWordDetails {
            key,
            token,
            encountered,
            candidates,
            custom_entry,
        }))
    }

    /// Read a currently resolved effective Japanese segment. The whole frozen
    /// resolution is compared before using its index: an override replacement,
    /// reversal or metadata change cannot silently select a different word.
    /// Status/count/custom-entry reads remain live and never register vocabulary.
    /// Only exact analyzer metadata may supply a base form for dictionary lookup.
    pub fn load_effective_reader_word(
        &mut self,
        dictionary: &Dictionary,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
    ) -> Result<Option<EffectiveReaderWordDetails>, ReaderWordError> {
        let transaction = self.connection.transaction().map_err(StorageError::from)?;
        let Some(snapshot) =
            load_effective_reader_tokens_in(&transaction, key.document_id, key.page)?
        else {
            return Ok(None);
        };
        if snapshot.segmentation.as_ref() != Some(&key.segmentation) {
            return Err(ReaderWordError::InvalidSelection);
        }
        let token = snapshot
            .tokens
            .into_iter()
            .find(|token| token.segment_index == segment_index)
            .ok_or(ReaderWordError::InvalidSelection)?;
        let (encountered, candidates, custom_entry) = word_details_in(
            &transaction,
            dictionary,
            &token.surface,
            token
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.dictionary_form.as_ref().map(|form| form.as_str())),
            token.vocabulary,
        )?;
        transaction.commit().map_err(StorageError::from)?;
        Ok(Some(EffectiveReaderWordDetails {
            key: key.clone(),
            token,
            encountered,
            candidates,
            custom_entry,
        }))
    }
}

pub(crate) fn selected_token_in(
    connection: &Connection,
    key: ReaderTokenKey,
    token_index: u64,
) -> Result<Option<ReaderToken>, ReaderWordError> {
    let Some(snapshot) = load_reader_tokens_in(connection, key.document_id, key.page)? else {
        return Ok(None);
    };
    if snapshot.key() != key {
        return Err(ReaderWordError::InvalidSelection);
    }
    snapshot
        .tokens
        .into_iter()
        .find(|token| token.token_index == token_index)
        .map(Some)
        .ok_or(ReaderWordError::InvalidSelection)
}

/// Identical exact-surface semantics for original and effective selections.
fn word_details_in(
    connection: &Connection,
    dictionary: &Dictionary,
    surface: &str,
    dictionary_form: Option<&str>,
    vocabulary: Option<ReaderVocabulary>,
) -> Result<(u64, DictionaryCandidates, Option<ReaderCustomEntry>), ReaderWordError> {
    let encountered = match vocabulary {
        Some(entry) => {
            let count: i64 = connection
                .query_row(COUNT_SQL, [i64::from(entry.id)], |row| row.get(0))
                .map_err(StorageError::from)?;
            u64::try_from(count).map_err(|_| StorageError::InvalidVocabulary)?
        }
        None => 0,
    };
    let candidates = dictionary.lookup(surface, dictionary_form)?;
    let custom_entry = if candidates.words.is_empty() && candidates.names.is_empty() {
        crate::custom_dictionary::read_entry_in(
            connection,
            yomoku_core::vocabulary::Surface::new(surface)
                .map_err(|_| ReaderWordError::InvalidSelection)?,
        )
        .map_err(|error| match error {
            crate::custom_dictionary::CustomDictionaryError::Storage(error) => {
                ReaderWordError::Storage(error)
            }
            _ => ReaderWordError::InvalidCustomEntry,
        })?
        .map(Into::into)
    } else {
        None
    };
    Ok((encountered, candidates, custom_entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestDatabase;

    #[test]
    fn lifetime_count_uses_existing_entry_index() {
        let database = TestDatabase::new();
        let storage = Storage::open(database.path()).unwrap();
        let plan: Vec<String> = storage
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {COUNT_SQL}"))
            .unwrap()
            .query_map([1_i64], |row| row.get(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(plan.iter().any(|line| line.contains("SEARCH reading_occurrences USING COVERING INDEX reading_occurrences_by_entry (vocabulary_id=?)")), "{plan:?}");
    }
}
