use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::document::{
    ChapterId, ChapterText, DocumentId, LibraryDocument, PasteText, PastedDocument, SourceId,
    TextOffset,
};
use yomoku_core::{
    pagination::ReadingPosition,
    reader::{progress_percent, ReaderSnapshot},
};

use crate::{analysis::read_reader_analysis, settings::read_page_length, Storage, StorageError};

impl Storage {
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

    /// Stable allocation order until Library sorting is implemented.
    pub fn list_documents(&self) -> Result<Vec<LibraryDocument>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT d.id, d.title, length(CAST(t.text AS BLOB)),
                             p.chapter_id, p.byte_offset, c.id, d.finished_at_least_once
                      FROM documents d
                      LEFT JOIN chapters c ON c.source_id = d.source_id AND c.chapter_index = 0
                      LEFT JOIN chapter_texts t ON t.chapter_id = c.id
                      LEFT JOIN reading_positions p ON p.document_id = d.id
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
            ))
        })?;
        rows.map(|row| {
            let (id, title, total_bytes, position_chapter, offset, chapter, finished_at_least_once) = row?;
            let total_bytes = total_bytes.ok_or(StorageError::InvalidDocument)?;
            let chapter = chapter.ok_or(StorageError::InvalidDocument)?;
            if position_chapter.is_some_and(|id| id != chapter) {
                return Err(StorageError::InvalidDocument);
            }
            Ok(LibraryDocument {
                document_id: DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?,
                title,
                finished_at_least_once,
                progress_percent: progress_percent(offset.unwrap_or(0), total_bytes)
                    .map_err(|_| StorageError::InvalidDocument)?,
            })
        })
        .collect()
    }

    /// Persist one paste atomically. This does not tokenize/register vocabulary,
    /// deduplicate imports or start reading; the runtime import pipeline adds those.
    /// There is deliberately no content-edit operation.
    pub fn create_paste(&mut self, text: &PasteText) -> Result<PastedDocument, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let document = create_paste_in(&transaction, text)?;
        transaction.commit()?;
        Ok(document)
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

    /// Read the exact position, preset and original stored Mode B identity
    /// together. This never invokes an analyzer, scans tokens or writes data.
    pub fn load_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<ReaderSnapshot>, StorageError> {
        let transaction = self.connection.transaction()?;
        let Some(document) = load_paste_in(&transaction, document_id)? else {
            return Ok(None);
        };
        let snapshot = ReaderSnapshot {
            page_length: read_page_length(&transaction)?,
            analysis: read_reader_analysis(&transaction, document.chapter.chapter_id)?,
            document,
        };
        transaction.commit()?;
        Ok(Some(snapshot))
    }

    /// Explicit reader entry. A finished book starts at page 1 with the reset
    /// persisted in the same transaction as the returned snapshot. Ordinary
    /// reads and completion reloads must keep using the read-only load methods.
    pub fn open_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<ReaderSnapshot>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut document) = load_paste_in(&transaction, document_id)? else {
            return Ok(None);
        };
        if document.finished_at_least_once {
            let start = ReadingPosition {
                source_id: document.chapter.source_id,
                chapter_id: document.chapter.chapter_id,
                offset: TextOffset::START,
            };
            if document.reading_position != Some(start) {
                write_position_in(&transaction, document_id, start)?;
                document.reading_position = Some(start);
            }
        }
        let snapshot = ReaderSnapshot {
            page_length: read_page_length(&transaction)?,
            analysis: read_reader_analysis(&transaction, document.chapter.chapter_id)?,
            document,
        };
        transaction.commit()?;
        Ok(Some(snapshot))
    }

    /// Save only current position. No completion/history/status changes occur.
    /// Validate ownership and scalar boundaries before replacing the previous row.
    pub fn save_reading_position(
        &mut self,
        document_id: DocumentId,
        position: ReadingPosition,
    ) -> Result<Option<PastedDocument>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut document) = load_paste_in(&transaction, document_id)? else {
            return Ok(None);
        };
        position
            .validate(&document.chapter)
            .map_err(|_| StorageError::InvalidDocument)?;
        write_position_in(&transaction, document_id, position)?;
        transaction.commit()?;
        document.reading_position = Some(position);
        Ok(Some(document))
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
                 WHERE sources.id = ?1 AND sources.kind = 'paste' AND chapters.chapter_index = 0",
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
    connection.execute(
        "INSERT INTO sources (kind, original_title) VALUES ('paste', ?1)",
        [title],
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
        (i64::from(chapter_id), text.as_str()),
    )?;
    Ok(PastedDocument {
        document_id,
        title: title.to_owned(),
        finished_at_least_once: false,
        reading_position: None,
        chapter: ChapterText {
            source_id,
            chapter_id,
            text: text.as_str().to_owned(),
        },
    })
}
