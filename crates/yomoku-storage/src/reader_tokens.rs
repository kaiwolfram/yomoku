use rusqlite::Connection;

use yomoku_core::{
    document::{DocumentId, TextAnchor, VocabularyId},
    reader::{reader_tokens_on_page, ReaderAnalysis, ReaderTokenSnapshot, ReaderVocabulary},
};

use crate::{
    analysis::{read_analysis, read_reader_analysis},
    reader_documents::load_reader_document_in,
    vocabulary::decode_status,
    Storage, StorageError,
};

impl Storage {
    /// Read original Mode B tokens intersecting a supplied page span and their
    /// exact-surface global statuses in one consistent transaction. This does not
    /// load an analyzer, apply boundary overrides, reset progress, register words
    /// or record encounters. None means the live document no longer exists.
    pub fn load_reader_tokens(
        &mut self,
        document_id: DocumentId,
        page: TextAnchor,
    ) -> Result<Option<ReaderTokenSnapshot>, StorageError> {
        let transaction = self.connection.transaction()?;
        let snapshot = load_reader_tokens_in(&transaction, document_id, page)?;
        transaction.commit()?;
        Ok(snapshot)
    }
}

/// Shared read transaction for page tokens and selected-word lifetime counts.
pub(crate) fn load_reader_tokens_in(
    connection: &Connection,
    document_id: DocumentId,
    page: TextAnchor,
) -> Result<Option<ReaderTokenSnapshot>, StorageError> {
    let Some(document) = load_reader_document_in(connection, document_id)? else {
        return Ok(None);
    };
    let chapter = document
        .chapter(page.chapter_id)
        .map_err(|_| StorageError::InvalidAnalysis)?;
    if page
        .slice(chapter)
        .map_err(|_| StorageError::InvalidAnalysis)?
        .is_empty()
    {
        return Err(StorageError::InvalidAnalysis);
    }
    let analysis = read_reader_analysis(connection, chapter.chapter_id)?;
    let mut tokens = Vec::new();
    if let ReaderAnalysis::Available(id) = analysis {
        let stored = read_analysis(connection, id)?.ok_or(StorageError::InvalidAnalysis)?;
        tokens = reader_tokens_on_page(chapter, &stored, page)
            .map_err(|_| StorageError::InvalidAnalysis)?;
        // One exact-surface join for the visible page, including repeated
        // spellings. Analysis has no vocabulary FK: deletion leaves tokens
        // readable with an absent entry rather than silently recreating New.
        let mut statement = connection.prepare(
            "SELECT t.token_index, v.id, v.status
                 FROM analyzed_tokens t LEFT JOIN vocabulary v ON v.surface = t.surface
                 WHERE t.analysis_id = ?1 AND t.is_japanese = 1
                   AND t.start_byte < ?2 AND t.end_byte > ?3
                 ORDER BY t.token_index",
        )?;
        let mut rows = statement.query((
            i64::from(id),
            i64::from(page.range.end),
            i64::from(page.range.start),
        ))?;
        for token in &mut tokens {
            let row = rows.next()?.ok_or(StorageError::InvalidAnalysis)?;
            if u64::try_from(row.get::<_, i64>(0)?).ok() != Some(token.token_index) {
                return Err(StorageError::InvalidAnalysis);
            }
            token.vocabulary = match (
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ) {
                (Some(id), Some(status)) => Some(ReaderVocabulary {
                    id: VocabularyId::try_from(id).map_err(|_| StorageError::InvalidVocabulary)?,
                    status: decode_status(&status)?,
                }),
                (None, None) => None,
                _ => return Err(StorageError::InvalidVocabulary),
            };
        }
        if rows.next()?.is_some() {
            return Err(StorageError::InvalidAnalysis);
        }
    }
    Ok(Some(ReaderTokenSnapshot {
        document_id,
        page,
        analysis,
        tokens,
    }))
}
