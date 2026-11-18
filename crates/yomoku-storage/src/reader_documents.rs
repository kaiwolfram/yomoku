//! Whole-document snapshots and position repositories for production reading.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    document::{ChapterId, ChapterText, DocumentId, ReaderDocument, TextOffset, TextRange},
    pagination::ReadingPosition,
    reader::{DocumentReaderSnapshot, ReaderChapterAnalysis},
    reader_formatting::{ReaderChapterFormatting, ReaderTextSpan, ReaderTextStyle},
    reader_images::{ReaderChapterImages, ReaderImageAsset, ReaderInlineImage},
};
use yomoku_import::epub::SpanKind;

use crate::{
    analysis::read_reader_analysis,
    documents::{
        load_paste_in, read_last_read_sequence_in, record_reader_entry_in, write_position_in,
    },
    epub::load_epub_metadata_in,
    reader_integrity::ReaderIntegrity,
    settings::read_page_length,
    Storage, StorageError,
};

impl Storage {
    /// Validated canonical chapter for lazy dictionary component analysis.
    pub fn load_reader_chapter(
        &mut self,
        document_id: DocumentId,
        chapter_id: ChapterId,
    ) -> Result<Option<ChapterText>, StorageError> {
        let transaction = self.connection.transaction()?;
        let chapter = load_reader_chapter_in(
            &transaction,
            document_id,
            chapter_id,
            &self.reader_integrity,
        )?;
        transaction.commit()?;
        Ok(chapter)
    }

    /// Read all canonical chapters, exact position, preset and original per-chapter
    /// Mode C identities atomically. Never parses source bytes or loads Full.
    pub fn load_document_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<DocumentReaderSnapshot>, StorageError> {
        let transaction = self.connection.transaction()?;
        let Some(document) =
            load_reader_document_in(&transaction, document_id, &self.reader_integrity)?
        else {
            return Ok(None);
        };
        let snapshot = snapshot_in(&transaction, document)?;
        transaction.commit()?;
        Ok(Some(snapshot))
    }

    /// Explicit entry records Library recency and resets a finished document to
    /// its first chapter's start. Read-only/completion loads retain both values.
    pub fn open_document_reader_snapshot(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<DocumentReaderSnapshot>, StorageError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(mut document) =
            load_reader_document_in(&transaction, document_id, &self.reader_integrity)?
        else {
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
        record_reader_entry_in(&transaction, document_id)?;
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
        let Some(mut document) =
            load_reader_document_in(&transaction, document_id, &self.reader_integrity)?
        else {
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
        last_read_sequence: read_last_read_sequence_in(connection, document.document_id)?,
        typography: crate::settings::read_reader_typography(connection)?,
        document,
        page_length: read_page_length(connection)?,
        analyses,
    })
}

pub(crate) fn load_reader_document_in(
    connection: &Connection,
    document_id: DocumentId,
    integrity: &ReaderIntegrity,
) -> Result<Option<ReaderDocument>, StorageError> {
    #[cfg(test)]
    crate::tests::reader_work::DOCUMENTS.update(|count| count + 1);
    let revision = integrity.revision(connection)?;
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
            integrity.validate_binaries(connection, document_id, revision)?;
            let stored = load_epub_metadata_in(connection, document_id)?
                .ok_or(StorageError::InvalidDocument)?;
            let formatting = stored
                .chapters
                .iter()
                .map(|item| {
                    let spans = item
                        .spans
                        .iter()
                        .map(|span| {
                            let style = match &span.kind {
                                SpanKind::Heading(level) => ReaderTextStyle::Heading(*level),
                                SpanKind::Emphasis => ReaderTextStyle::Emphasis,
                                SpanKind::Strong => ReaderTextStyle::Strong,
                                SpanKind::ExternalLink(target) => {
                                    ReaderTextStyle::ExternalLink(target.clone())
                                }
                            };
                            Ok(ReaderTextSpan {
                                range: TextRange {
                                    start: TextOffset::at(&item.chapter.text, span.range.start)?,
                                    end: TextOffset::at(&item.chapter.text, span.range.end)?,
                                },
                                style,
                            })
                        })
                        .collect::<Result<Vec<_>, yomoku_core::document::InvalidTextValue>>()
                        .map_err(|_| StorageError::InvalidDocument)?;
                    Ok(ReaderChapterFormatting {
                        chapter_id: item.chapter.chapter_id,
                        spans,
                    })
                })
                .collect::<Result<_, StorageError>>()?;
            let images = stored
                .chapters
                .iter()
                .filter(|item| !item.images.is_empty())
                .map(|item| {
                    let images = item
                        .images
                        .iter()
                        .map(|image| {
                            let asset = stored
                                .images
                                .get(&image.path)
                                .ok_or(StorageError::InvalidDocument)?;
                            Ok(ReaderInlineImage {
                                range: TextRange {
                                    start: TextOffset::at(&item.chapter.text, image.range.start)
                                        .map_err(|_| StorageError::InvalidDocument)?,
                                    end: TextOffset::at(&item.chapter.text, image.range.end)
                                        .map_err(|_| StorageError::InvalidDocument)?,
                                },
                                asset: ReaderImageAsset {
                                    path: image.path.clone(),
                                    media_type: asset.clone(),
                                },
                                alt: image.alt.clone(),
                            })
                        })
                        .collect::<Result<_, StorageError>>()?;
                    Ok(ReaderChapterImages {
                        chapter_id: item.chapter.chapter_id,
                        images,
                    })
                })
                .collect::<Result<_, StorageError>>()?;
            ReaderDocument {
                document_id,
                title: stored.title,
                finished_at_least_once: stored.finished_at_least_once,
                read_override: stored.read_override,
                reading_position: stored.reading_position,
                formatting,
                images,
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
            let document: ReaderDocument = load_paste_in(connection, document_id)?
                .ok_or(StorageError::InvalidDocument)?
                .into();
            document
        }
        _ => return Err(StorageError::InvalidDocument),
    };
    document
        .validate()
        .map_err(|_| StorageError::InvalidDocument)?;
    if kind.as_deref() == Some("epub") {
        integrity.remember_document(document_id, document.chapters[0].source_id, revision);
    }
    Ok(Some(document))
}

/// Reuse whole-document integrity only at the exact database revision. On a hit,
/// retrieve just the requested canonical chapter; no other text or annotations.
pub(crate) fn load_reader_chapter_in(
    connection: &Connection,
    document_id: DocumentId,
    chapter_id: ChapterId,
    integrity: &ReaderIntegrity,
) -> Result<Option<ChapterText>, StorageError> {
    let revision = integrity.revision(connection)?;
    if let Some(source_id) = integrity.validated_source(document_id, revision) {
        let chapter = crate::analysis::read_chapter(connection, chapter_id)?;
        if chapter.source_id != source_id {
            return Err(StorageError::InvalidAnalysis);
        }
        return Ok(Some(chapter));
    }
    let Some(document) = load_reader_document_in(connection, document_id, integrity)? else {
        return Ok(None);
    };
    document
        .chapters
        .into_iter()
        .find(|chapter| chapter.chapter_id == chapter_id)
        .map(Some)
        .ok_or(StorageError::InvalidAnalysis)
}
