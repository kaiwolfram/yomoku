//! Whole-document reader repositories, independent of the legacy bridge payload.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    document::{DocumentId, ReaderDocument, TextOffset},
    pagination::ReadingPosition,
    reader::{DocumentReaderSnapshot, ReaderChapterAnalysis},
};

use crate::{
    analysis::read_reader_analysis,
    documents::{load_paste_in, write_position_in},
    epub::load_epub_in,
    settings::read_page_length,
    Storage, StorageError,
};

impl Storage {
    /// Read all canonical chapters, exact position, preset and original per-chapter
    /// Mode B identities atomically. Never parses source bytes or loads Full.
    pub fn load_document_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<DocumentReaderSnapshot>, StorageError> {
        let transaction = self.connection.transaction()?;
        let Some(document) = load_reader_document_in(&transaction, document_id)? else {
            return Ok(None);
        };
        let snapshot = snapshot_in(&transaction, document)?;
        transaction.commit()?;
        Ok(Some(snapshot))
    }

    /// Explicit entry resets a finished document to its first chapter's start.
    /// Read-only loads and completion reloads retain the saved position.
    pub fn open_document_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<DocumentReaderSnapshot>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut document) = load_reader_document_in(&transaction, document_id)? else {
            return Ok(None);
        };
        if document.finished_at_least_once {
            let first = &document.chapters[0];
            let start = ReadingPosition {
                source_id: first.source_id,
                chapter_id: first.chapter_id,
                offset: TextOffset::START,
            };
            if document.reading_position != Some(start) {
                write_position_in(&transaction, document_id, start)?;
                document.reading_position = Some(start);
            }
        }
        let snapshot = snapshot_in(&transaction, document)?;
        transaction.commit()?;
        Ok(Some(snapshot))
    }

    /// Position-only movement across any owned chapter, with no reading events.
    pub fn save_document_reading_position(
        &mut self,
        document_id: DocumentId,
        position: ReadingPosition,
    ) -> Result<Option<ReaderDocument>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut document) = load_reader_document_in(&transaction, document_id)? else {
            return Ok(None);
        };
        position
            .validate(
                document
                    .chapter(position.chapter_id)
                    .map_err(|_| StorageError::InvalidDocument)?,
            )
            .map_err(|_| StorageError::InvalidDocument)?;
        write_position_in(&transaction, document_id, position)?;
        transaction.commit()?;
        document.reading_position = Some(position);
        Ok(Some(document))
    }
}

fn snapshot_in(
    connection: &Connection,
    document: ReaderDocument,
) -> Result<DocumentReaderSnapshot, StorageError> {
    let analyses = document
        .chapters
        .iter()
        .map(|chapter| {
            Ok(ReaderChapterAnalysis {
                chapter_id: chapter.chapter_id,
                analysis: read_reader_analysis(connection, chapter.chapter_id)?,
            })
        })
        .collect::<Result<_, StorageError>>()?;
    Ok(DocumentReaderSnapshot {
        document,
        page_length: read_page_length(connection)?,
        analyses,
    })
}

pub(crate) fn load_reader_document_in(
    connection: &Connection,
    document_id: DocumentId,
) -> Result<Option<ReaderDocument>, StorageError> {
    // Query the live document first: absent provenance for a live row is an
    // error, not a deleted-document result or a fallback to chapter zero.
    let kind: Option<Option<String>> = connection
        .query_row(
            "SELECT s.kind FROM documents d LEFT JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
            [i64::from(document_id)],
            |row| row.get(0),
        )
        .optional()?;
    let Some(kind) = kind else {
        return Ok(None);
    };
    let document = match kind.as_deref() {
        Some("epub") => {
            // Reuse the authoritative read-only integrity checks for the saved
            // container, chapter counts/order, annotations and assets. Source
            // bytes and image payloads are not part of the text projection.
            let stored =
                load_epub_in(connection, document_id)?.ok_or(StorageError::InvalidDocument)?;
            ReaderDocument {
                document_id,
                title: stored.title,
                finished_at_least_once: stored.finished_at_least_once,
                reading_position: stored.reading_position,
                chapters: stored
                    .chapters
                    .into_iter()
                    .map(|item| item.chapter)
                    .collect(),
            }
        }
        Some("paste" | "txt" | "srt") => {
            // Reject extra/missing units rather than silently truncate a damaged
            // single-text source through the historical chapter-zero loader.
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM chapters c JOIN documents d ON d.source_id = c.source_id WHERE d.id = ?1",
                [i64::from(document_id)],
                |row| row.get(0),
            )?;
            if count != 1 {
                return Err(StorageError::InvalidDocument);
            }
            load_paste_in(connection, document_id)?
                .ok_or(StorageError::InvalidDocument)?
                .into()
        }
        _ => return Err(StorageError::InvalidDocument),
    };
    document
        .validate()
        .map_err(|_| StorageError::InvalidDocument)?;
    Ok(Some(document))
}
