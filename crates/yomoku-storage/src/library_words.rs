//! Read-only Library distribution over the same frozen B used by the live reader.

use yomoku_core::{
    document::DocumentId,
    library::{LibraryWordDistribution, WordStatusCounts},
};

use crate::{Storage, StorageError};

impl Storage {
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
                    WHERE chapter_id = c.id AND split_mode = 'b'
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
