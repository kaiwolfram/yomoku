//! Read-only Library distributions for original-C and effective reader membership.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use yomoku_core::{
    document::{ChapterId, DocumentId},
    library::{LibraryWordDistribution, WordStatusCounts},
    vocabulary::{Surface, VocabularyStatus},
};

use crate::{
    analysis::{read_chapter, read_effective_reader_chapter_in},
    vocabulary::read_entry_by_surface,
    Storage, StorageError,
};

impl Storage {
    /// Resolve all live documents and statuses in one read transaction. This
    /// separate API is ready for the coordinated effective-reader switch; the
    /// live B reader keeps its matching original-C distribution until then.
    /// Reads canonical chapters/stored analysis/rules, never source/assets or
    /// dictionaries. Missing B stays unavailable; missing vocabulary stays absent.
    pub fn effective_library_word_distributions(
        &mut self,
    ) -> Result<Vec<LibraryWordDistribution>, StorageError> {
        let transaction = self.connection.transaction()?;
        let documents = {
            let mut statement =
                transaction.prepare("SELECT id, source_id FROM documents ORDER BY id")?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut distributions = Vec::with_capacity(documents.len());
        // Exact spellings share a global status, and this transaction pins one
        // snapshot. Resolve a referenced spelling only once per scan, including
        // absent entries. Nothing survives commit, failure or a later refresh.
        let mut statuses = BTreeMap::new();
        for (id, source_id) in documents {
            let chapters = {
                let mut statement = transaction.prepare(
                    "SELECT id FROM chapters WHERE source_id = ?1 ORDER BY chapter_index",
                )?;
                let rows = statement.query_map([source_id], |row| row.get::<_, i64>(0))?;
                rows.collect::<Result<Vec<_>, _>>()?
            };
            if chapters.is_empty() {
                return Err(StorageError::InvalidAnalysis);
            }
            let mut available = true;
            let mut surfaces = BTreeSet::new();
            for chapter_id in chapters {
                let chapter = read_chapter(
                    &transaction,
                    ChapterId::try_from(chapter_id).map_err(|_| StorageError::InvalidAnalysis)?,
                )?;
                let Some(effective) = read_effective_reader_chapter_in(&transaction, &chapter)?
                else {
                    available = false;
                    continue;
                };
                for surface in effective.eligible_surfaces() {
                    surfaces
                        .insert(Surface::new(surface).map_err(|_| StorageError::InvalidAnalysis)?);
                }
            }
            let mut counts = WordStatusCounts::default();
            if available {
                for surface in surfaces {
                    let status = match statuses.entry(surface) {
                        Entry::Occupied(entry) => *entry.get(),
                        Entry::Vacant(entry) => {
                            let status = read_entry_by_surface(&transaction, entry.key())?
                                .map(|entry| entry.status);
                            *entry.insert(status)
                        }
                    };
                    let Some(status) = status else {
                        continue;
                    };
                    let count = match status {
                        VocabularyStatus::New => &mut counts.new,
                        VocabularyStatus::Learning => &mut counts.learning,
                        VocabularyStatus::Familiar => &mut counts.familiar,
                        VocabularyStatus::Known => &mut counts.known,
                        VocabularyStatus::Ignored => &mut counts.ignored,
                    };
                    *count = count
                        .checked_add(1)
                        .ok_or(StorageError::InvalidVocabulary)?;
                }
            }
            distributions.push(LibraryWordDistribution {
                document_id: DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?,
                counts: available.then_some(counts),
            });
        }
        transaction.commit()?;
        Ok(distributions)
    }

    /// One SQLite statement/snapshot for every live document and global status.
    /// Deduplicate exact spellings across repetitions and chapters before joining
    /// vocabulary. Do not analyze, register missing entries or read source/assets.
    pub fn library_word_distributions(&self) -> Result<Vec<LibraryWordDistribution>, StorageError> {
        let mut statement = self.connection.prepare(
            "WITH original AS (
                SELECT c.source_id, a.id AS analysis_id, a.semantics_version,
                       a.engine_revision, a.dictionary_revision
                FROM chapters c
                LEFT JOIN chapter_analyses a ON a.id = (
                    SELECT MIN(id) FROM chapter_analyses
                    WHERE chapter_id = c.id AND split_mode = 'c'
                )
              ), coverage AS (
                SELECT source_id, COUNT(*) = COUNT(analysis_id) AS available,
                       MIN(analysis_id IS NULL OR (semantics_version = 1
                           AND length(trim(engine_revision)) > 0
                           AND length(trim(dictionary_revision)) > 0)) AS valid
                FROM original GROUP BY source_id
              ), surfaces AS (
                SELECT DISTINCT o.source_id, t.surface
                FROM original o JOIN analyzed_tokens t ON t.analysis_id = o.analysis_id
                WHERE t.is_japanese = 1
              ), counts AS (
                SELECT s.source_id,
                       SUM(v.status = 'new') AS new,
                       SUM(v.status = 'learning') AS learning,
                       SUM(v.status = 'familiar') AS familiar,
                       SUM(v.status = 'known') AS known,
                       SUM(v.status = 'ignored') AS ignored
                FROM surfaces s JOIN vocabulary v ON v.surface = s.surface
                GROUP BY s.source_id
              )
              SELECT d.id, c.available, c.valid,
                     COALESCE(n.new, 0), COALESCE(n.learning, 0),
                     COALESCE(n.familiar, 0), COALESCE(n.known, 0), COALESCE(n.ignored, 0)
              FROM documents d LEFT JOIN coverage c ON c.source_id = d.source_id
              LEFT JOIN counts n ON n.source_id = d.source_id ORDER BY d.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<bool>>(1)?,
                row.get::<_, Option<bool>>(2)?,
                [
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ],
            ))
        })?;
        rows.map(|row| {
            let (id, available, valid, counts) = row?;
            if valid != Some(true) {
                return Err(StorageError::InvalidAnalysis);
            }
            let [new, learning, familiar, known, ignored] = counts.map(u64::try_from);
            let counts = WordStatusCounts {
                new: new.map_err(|_| StorageError::InvalidVocabulary)?,
                learning: learning.map_err(|_| StorageError::InvalidVocabulary)?,
                familiar: familiar.map_err(|_| StorageError::InvalidVocabulary)?,
                known: known.map_err(|_| StorageError::InvalidVocabulary)?,
                ignored: ignored.map_err(|_| StorageError::InvalidVocabulary)?,
            };
            Ok(LibraryWordDistribution {
                document_id: DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?,
                counts: available.unwrap_or(false).then_some(counts),
            })
        })
        .collect()
    }
}
