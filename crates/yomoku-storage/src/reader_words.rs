//! Selected-word read composition for the runtime's I/O worker. Dictionary
//! assets remain separate from mutable state; neither connection is written.

use std::{error::Error, fmt, sync::Arc};

use rusqlite::Connection;
use yomoku_core::{
    dictionary::DictionaryCandidates,
    effective_reader::{EffectiveReaderToken, EffectiveReaderTokenKey, EffectiveReaderWordDetails},
    reader::ReaderVocabulary,
    vocabulary::Surface,
};

use crate::{
    dictionary::{Dictionary, DictionaryError},
    reader_integrity::{ActiveChapter, ReaderIntegrity},
    Storage, StorageError,
};

const COUNT_SQL: &str = "SELECT COUNT(*), MAX(c.completed_at_unix_ms)
    FROM reading_occurrences o LEFT JOIN reading_completions c USING (command_id)
    WHERE o.vocabulary_id = ?1";

mod selection_cache;
pub(crate) use selection_cache::{SelectionRevision, ValidatedSelection};

#[derive(Debug)]
pub enum ReaderWordError {
    Storage(StorageError),
    Dictionary(DictionaryError),
    /// A stale base identity or an index outside this page's Japanese tokens.
    InvalidSelection,
}

impl fmt::Display for ReaderWordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => error.fmt(f),
            Self::Dictionary(error) => error.fmt(f),
            Self::InvalidSelection => f.write_str("invalid or stale reader word selection"),
        }
    }
}

impl Error for ReaderWordError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Dictionary(error) => Some(error),
            Self::InvalidSelection => None,
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
    /// Read a currently resolved effective Japanese segment. The whole frozen
    /// resolution is compared before using its index, so metadata changes cannot
    /// silently select a different word.
    /// Status/count reads remain live and never register vocabulary.
    /// Only exact analyzer metadata may supply a base form for dictionary lookup.
    pub fn load_effective_reader_word(
        &mut self,
        dictionary: &Dictionary,
        key: &EffectiveReaderTokenKey,
        segment_index: u64,
    ) -> Result<Option<EffectiveReaderWordDetails>, ReaderWordError> {
        self.validated_reader_selection = None;
        let transaction = self.connection.transaction().map_err(StorageError::from)?;
        let revision = SelectionRevision::read(&transaction)?;
        let Some((chapter, mut token)) =
            selected_effective_token_in(&transaction, key, segment_index, &self.reader_integrity)?
        else {
            return Ok(None);
        };
        let surface = Surface::new(&token.surface).map_err(|_| StorageError::InvalidAnalysis)?;
        token.vocabulary =
            crate::vocabulary::read_entry_by_surface(&transaction, &surface)?.map(|entry| {
                ReaderVocabulary {
                    id: entry.id,
                    status: entry.status,
                }
            });
        let components = self
            .reader_integrity
            .components(|| component_analyses_in(&transaction, key, &chapter.text))?;
        let anchor = token.anchor;
        let forms = yomoku_core::dictionary::component_parts(
            components
                .iter()
                .flat_map(|analysis| {
                    let start = analysis
                        .tokens
                        .partition_point(|t| t.anchor.range.start < anchor.range.start);
                    analysis.tokens[start..]
                        .iter()
                        .take_while(|t| t.anchor.range.end <= anchor.range.end)
                        .cloned()
                })
                .collect(),
            anchor.range.start,
        );
        let parts = dictionary.parts(&token.surface, forms)?;
        let (encountered, last_seen_unix_ms, candidates) = word_details_in(
            &transaction,
            dictionary,
            &token.surface,
            token
                .metadata
                .dictionary_form
                .as_ref()
                .map(|form| form.as_str()),
            token.vocabulary,
        )?;
        transaction.commit().map_err(StorageError::from)?;
        self.validated_reader_selection =
            Some(ValidatedSelection::new(key, token.clone(), revision));
        Ok(Some(EffectiveReaderWordDetails {
            key: key.clone(),
            parts,
            token,
            encountered,
            last_seen_unix_ms,
            candidates,
        }))
    }
}

/// Shared validation for effective word reads. Call
/// inside the operation's transaction so rules cannot change between validation
/// and use. Only visible, eligible segments may supply surface/base identity.
pub(crate) fn selected_effective_token_in(
    connection: &Connection,
    key: &EffectiveReaderTokenKey,
    segment_index: u64,
    integrity: &ReaderIntegrity,
) -> Result<Option<(Arc<ActiveChapter>, EffectiveReaderToken)>, ReaderWordError> {
    let Some(chapter) = integrity.chapter(connection, key.document_id, key.page.chapter_id)? else {
        return Ok(None);
    };
    let effective = chapter
        .effective
        .as_ref()
        .filter(|effective| effective.segmentation() == &key.segmentation)
        .ok_or(ReaderWordError::InvalidSelection)?;
    let token = effective
        .token_on_page(key.page, segment_index)
        .map_err(|_| StorageError::InvalidAnalysis)?
        .ok_or(ReaderWordError::InvalidSelection)?;
    Ok(Some((chapter, token)))
}

/// Exact-surface counts and bundled dictionary candidates for a validated selection.
fn word_details_in(
    connection: &Connection,
    dictionary: &Dictionary,
    surface: &str,
    dictionary_form: Option<&str>,
    vocabulary: Option<ReaderVocabulary>,
) -> Result<(u64, Option<i64>, DictionaryCandidates), ReaderWordError> {
    let (encountered, last_seen_unix_ms) = match vocabulary {
        Some(entry) => {
            let (count, last_seen): (i64, Option<i64>) = connection
                .query_row(COUNT_SQL, [i64::from(entry.id)], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(StorageError::from)?;
            if (count == 0) != last_seen.is_none() || last_seen.is_some_and(|date| date < 0) {
                return Err(StorageError::InvalidVocabulary.into());
            }
            (
                u64::try_from(count).map_err(|_| StorageError::InvalidVocabulary)?,
                last_seen,
            )
        }
        None => (0, None),
    };
    let candidates = dictionary.lookup(surface, dictionary_form)?;
    Ok((encountered, last_seen_unix_ms, candidates))
}

fn component_analyses_in(
    connection: &Connection,
    key: &EffectiveReaderTokenKey,
    chapter: &yomoku_core::document::ChapterText,
) -> Result<Vec<yomoku_core::analysis::ChapterAnalysis>, StorageError> {
    let mut statement = connection.prepare("SELECT id FROM chapter_analyses WHERE chapter_id = ?1 AND engine_revision = ?2 AND dictionary_revision = ?3 AND split_mode IN ('a','b') ORDER BY split_mode")?;
    let ids = statement
        .query_map(
            (
                i64::from(chapter.chapter_id),
                &key.segmentation.profile.engine_revision,
                &key.segmentation.profile.dictionary_revision,
            ),
            |row| row.get::<_, i64>(0),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut components = Vec::new();
    for id in ids {
        let id = id.try_into().map_err(|_| StorageError::InvalidAnalysis)?;
        components.push(
            crate::analysis::read_analysis_for_chapter(connection, id, Some(chapter))?
                .ok_or(StorageError::InvalidAnalysis)?,
        );
    }
    Ok(components)
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
        assert!(
            plan.iter().any(|line| line.contains(
                "SEARCH o USING COVERING INDEX reading_occurrences_by_entry (vocabulary_id=?)"
            )),
            "{plan:?}"
        );
    }
}
