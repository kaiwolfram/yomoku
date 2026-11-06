use sha2::{Digest, Sha256};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::document::{
    ChapterId, ChapterText, DocumentId, LibraryDocument, PasteText, PastedDocument, SourceId,
    TextOffset,
};
use yomoku_core::{pagination::ReadingPosition, reader::progress_percent};

use crate::{Storage, StorageError};

impl Storage {
    /// Change only the live Library title, preserving literal spelling and all
    /// source/history/content/progress fields. False means the live ID is absent.
    /// Same-title retries perform no writes, including after a lost response.
    pub fn rename_document(
        &mut self,
        document_id: DocumentId,
        title: &str,
    ) -> Result<bool, StorageError> {
        if !yomoku_core::library::valid_document_title(title) {
            return Err(StorageError::InvalidDocument);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<String> = transaction
            .query_row(
                "SELECT title FROM documents WHERE id = ?1",
                [i64::from(document_id)],
                |row| row.get(0),
            )
            .optional()?;
        let Some(previous) = previous else {
            return Ok(false);
        };
        if previous != title {
            transaction.execute(
                "UPDATE documents SET title = ?2 WHERE id = ?1",
                rusqlite::params![i64::from(document_id), title],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    /// Remove live content and Library state atomically, retaining source/chapter
    /// provenance, global vocabulary and completed-reading history.
    /// An absent ID is an idempotent no-op, including an explicit lost-response retry.
    pub fn delete_document(&mut self, document_id: DocumentId) -> Result<(), StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        {
            let mut delete_text = transaction.prepare(
                "DELETE FROM chapter_texts WHERE chapter_id IN (
                SELECT c.id FROM chapters c JOIN documents d ON d.source_id = c.source_id
                WHERE d.id = ?1)",
            )?;
            // Existing foreign keys remove positions, file bytes,
            // dedup identities and EPUB assets. Text deletion removes live analyses.
            let mut delete_document = transaction.prepare("DELETE FROM documents WHERE id = ?1")?;
            delete_text.execute([i64::from(document_id)])?;
            delete_document.execute([i64::from(document_id)])?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Read recency for a live document, without opening/resetting it.
    pub fn last_read_sequence(&self, document_id: DocumentId) -> Result<Option<i64>, StorageError> {
        read_last_read_sequence_in(&self.connection, document_id)
    }

    /// Permanent finish state of a live document, independent of its position.
    /// None means the document is absent. Reading this never changes progress.
    pub fn finished_at_least_once(
        &self,
        document_id: DocumentId,
    ) -> Result<Option<bool>, StorageError> {
        self.connection
            .query_row(
                "SELECT finished_at_least_once FROM documents WHERE id = ?1",
                [i64::from(document_id)],
                |row| row.get(0),
            )
            .optional()
            .map_err(StorageError::from)
    }

    /// Read-only summaries in stable import order; the core projects Library sorts.
    pub fn list_documents(&self) -> Result<Vec<LibraryDocument>, StorageError> {
        let mut statement = self.connection.prepare(
            "WITH chapter_lengths AS (
                SELECT c.id, c.source_id, c.chapter_index,
                       length(CAST(t.text AS BLOB)) AS bytes
                FROM chapters c LEFT JOIN chapter_texts t ON t.chapter_id = c.id
              ), totals AS (
                SELECT source_id, SUM(bytes) AS bytes,
                       COUNT(*) = COUNT(bytes) AND MIN(chapter_index) = 0
                         AND MAX(chapter_index) = COUNT(*) - 1 AS complete
                FROM chapter_lengths GROUP BY source_id
              )
              SELECT d.id, d.title, totals.bytes, p.chapter_id, p.byte_offset,
                     c.bytes, d.finished_at_least_once, d.source_id, c.source_id,
                     totals.complete,
                     (SELECT COALESCE(SUM(prior.bytes), 0) FROM chapter_lengths prior
                      WHERE prior.source_id = d.source_id AND prior.chapter_index < c.chapter_index),
                     d.last_read_sequence
              FROM documents d
              LEFT JOIN totals ON totals.source_id = d.source_id
              LEFT JOIN reading_positions p ON p.document_id = d.id
              LEFT JOIN chapter_lengths c ON c.id = p.chapter_id
              ORDER BY d.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<bool>>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, Option<i64>>(11)?,
            ))
        })?;
        rows.map(|row| {
            let (
                id,
                title,
                total_bytes,
                position_chapter,
                offset,
                chapter_bytes,
                finished_at_least_once,
                source,
                position_source,
                complete,
                preceding_bytes,
                last_read_sequence,
            ) = row?;
            let total_bytes = total_bytes.ok_or(StorageError::InvalidDocument)?;
            if last_read_sequence.is_some_and(|value| value <= 0)
                || complete != Some(true)
                || position_chapter.is_some()
                    && (position_source != Some(source)
                        || !matches!((offset, chapter_bytes), (Some(offset), Some(bytes)) if (0..=bytes).contains(&offset)))
            {
                return Err(StorageError::InvalidDocument);
            }
            let offset = preceding_bytes
                .checked_add(offset.unwrap_or(0))
                .ok_or(StorageError::InvalidDocument)?;
            Ok(LibraryDocument {
                last_read_sequence,
                document_id: DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?,
                title,
                finished_at_least_once,
                progress_percent: progress_percent(offset, total_bytes)
                    .map_err(|_| StorageError::InvalidDocument)?,
            })
        })
        .collect()
    }

    /// Read a consistent snapshot. None means the Library document is absent;
    /// missing provenance/text for an existing document is an error.
    pub fn load_paste(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<PastedDocument>, StorageError> {
        let transaction = self.connection.transaction()?;
        let document = load_paste_in(&transaction, document_id)?;
        transaction.commit()?;
        Ok(document)
    }
}

pub(crate) fn write_position_in(
    connection: &Connection,
    document_id: DocumentId,
    position: ReadingPosition,
) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO reading_positions (document_id, chapter_id, byte_offset) VALUES (?1, ?2, ?3)
         ON CONFLICT(document_id) DO UPDATE SET chapter_id = excluded.chapter_id, byte_offset = excluded.byte_offset",
        (i64::from(document_id), i64::from(position.chapter_id), i64::from(position.offset)),
    )?;
    Ok(())
}

pub(crate) fn load_paste_in(
    connection: &Connection,
    document_id: DocumentId,
) -> Result<Option<PastedDocument>, StorageError> {
    let document = connection
        .query_row(
            "SELECT source_id, title, finished_at_least_once FROM documents WHERE id = ?1",
            [i64::from(document_id)],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((source_id, title, finished_at_least_once)) = document else {
        return Ok(None);
    };
    let source_id = SourceId::try_from(source_id).map_err(|_| StorageError::InvalidDocument)?;
    let chapter = connection
        .query_row(
            "SELECT chapters.id, chapter_texts.text
                 FROM sources
                 JOIN chapters ON chapters.source_id = sources.id
                 JOIN chapter_texts ON chapter_texts.chapter_id = chapters.id
                 WHERE sources.id = ?1 AND sources.kind IN ('paste', 'txt', 'srt') AND chapters.chapter_index = 0",
            [i64::from(source_id)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or(StorageError::InvalidDocument)?;
    let chapter_id = ChapterId::try_from(chapter.0).map_err(|_| StorageError::InvalidDocument)?;
    let text = PasteText::new(chapter.1).map_err(|_| StorageError::InvalidDocument)?;
    let stored_position = connection
        .query_row(
            "SELECT chapter_id, byte_offset FROM reading_positions WHERE document_id = ?1",
            [i64::from(document_id)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    let reading_position = stored_position
        .map(|(chapter, offset)| {
            Ok::<_, StorageError>(ReadingPosition {
                source_id,
                chapter_id: ChapterId::try_from(chapter)
                    .map_err(|_| StorageError::InvalidDocument)?,
                offset: TextOffset::try_from(offset).map_err(|_| StorageError::InvalidDocument)?,
            })
        })
        .transpose()?;
    let document = PastedDocument {
        document_id,
        title,
        finished_at_least_once,
        reading_position,
        chapter: ChapterText {
            source_id,
            chapter_id,
            text: text.as_str().to_owned(),
        },
    };
    if let Some(position) = reading_position {
        position
            .validate(&document.chapter)
            .map_err(|_| StorageError::InvalidDocument)?;
    }
    Ok(Some(document))
}

/// Caller owns the transaction so analyzed imports can commit all rows together.
pub(crate) fn create_paste_in(
    connection: &Connection,
    text: &PasteText,
) -> Result<PastedDocument, StorageError> {
    let title = text.initial_title();
    let document = create_single_text_in(connection, "paste", title, text.as_str())?;
    let digest = Sha256::digest(text.as_str().as_bytes());
    connection.execute(
        "INSERT INTO paste_content_hashes (document_id, sha256) VALUES (?1, ?2)",
        (i64::from(document.document_id), digest.as_slice()),
    )?;
    Ok(document)
}

pub(crate) fn create_single_text_in(
    connection: &Connection,
    kind: &str,
    title: &str,
    text: &str,
) -> Result<PastedDocument, StorageError> {
    connection.execute(
        "INSERT INTO sources (kind, original_title) VALUES (?1, ?2)",
        (kind, title),
    )?;
    let source_id = SourceId::try_from(connection.last_insert_rowid())
        .map_err(|_| StorageError::InvalidDocument)?;
    connection.execute(
        "INSERT INTO chapters (source_id, chapter_index) VALUES (?1, 0)",
        [i64::from(source_id)],
    )?;
    let chapter_id = ChapterId::try_from(connection.last_insert_rowid())
        .map_err(|_| StorageError::InvalidDocument)?;
    connection.execute(
        "INSERT INTO documents (source_id, title) VALUES (?1, ?2)",
        (i64::from(source_id), title),
    )?;
    let document_id = DocumentId::try_from(connection.last_insert_rowid())
        .map_err(|_| StorageError::InvalidDocument)?;
    connection.execute(
        "INSERT INTO chapter_texts (chapter_id, text) VALUES (?1, ?2)",
        (i64::from(chapter_id), text),
    )?;
    Ok(PastedDocument {
        document_id,
        title: title.to_owned(),
        finished_at_least_once: false,
        reading_position: None,
        chapter: ChapterText {
            source_id,
            chapter_id,
            text: text.to_owned(),
        },
    })
}

/// Caller holds an immediate transaction. Recency commits with validated reader entry.
pub(crate) fn record_reader_entry_in(
    connection: &Connection,
    document_id: DocumentId,
) -> Result<(), StorageError> {
    if connection.execute(
        "UPDATE library_read_clock SET sequence = sequence + 1 WHERE id = 1 AND sequence < 9223372036854775807",
        [],
    )? != 1 {
        return Err(StorageError::InvalidDocument);
    }
    if connection.execute(
        "UPDATE documents SET last_read_sequence = (SELECT sequence FROM library_read_clock WHERE id = 1) WHERE id = ?1",
        [i64::from(document_id)],
    )? != 1 {
        return Err(StorageError::InvalidDocument);
    }
    Ok(())
}

pub(crate) fn read_last_read_sequence_in(
    connection: &Connection,
    document_id: DocumentId,
) -> Result<Option<i64>, StorageError> {
    let value: Option<i64> = connection.query_row(
        "SELECT last_read_sequence FROM documents WHERE id = ?1",
        [i64::from(document_id)],
        |row| row.get(0),
    )?;
    if value.is_some_and(|value| value <= 0) {
        return Err(StorageError::InvalidDocument);
    }
    Ok(value)
}
