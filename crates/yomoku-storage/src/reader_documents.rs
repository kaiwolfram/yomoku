//! Whole-document reader repositories, independent of the legacy bridge payload.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use yomoku_core::{
    document::{DocumentId, ReaderDocument, TextOffset, TextRange},
    pagination::ReadingPosition,
    reader::{DocumentReaderSnapshot, ReaderChapterAnalysis},
    reader_formatting::{ReaderChapterFormatting, ReaderTextSpan, ReaderTextStyle},
    reader_images::{ReaderChapterImages, ReaderImageAsset, ReaderInlineImage},
    reader_ruby::{ReaderChapterRuby, ReaderRuby},
};
use yomoku_import::epub::SpanKind;

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
        typography: crate::settings::read_reader_typography(connection)?,
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
            // bytes and image payloads are not part of the reader projection.
            let stored =
                load_epub_in(connection, document_id)?.ok_or(StorageError::InvalidDocument)?;
            let formatting = stored
                .chapters
                .iter()
                .map(|item| {
                    let spans = item
                        .spans
                        .iter()
                        .filter_map(|span| {
                            let style = match &span.kind {
                                SpanKind::Heading(level) => ReaderTextStyle::Heading(*level),
                                SpanKind::Emphasis => ReaderTextStyle::Emphasis,
                                SpanKind::Strong => ReaderTextStyle::Strong,
                                SpanKind::ExternalLink(target) => {
                                    ReaderTextStyle::ExternalLink(target.clone())
                                }
                                // Internal/other links do not acquire navigation or styling.
                                SpanKind::InertLink(_) => return None,
                            };
                            Some((|| {
                                Ok(ReaderTextSpan {
                                    range: TextRange {
                                        start: TextOffset::at(
                                            &item.chapter.text,
                                            span.range.start,
                                        )?,
                                        end: TextOffset::at(&item.chapter.text, span.range.end)?,
                                    },
                                    style,
                                })
                            })())
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
                                    media_type: asset.media_type.clone(),
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
                reading_position: stored.reading_position,
                formatting,
                images,
                source_ruby: stored
                    .chapters
                    .iter()
                    .filter(|item| !item.ruby.is_empty())
                    .map(|item| project_ruby(&item.chapter, &item.ruby))
                    .collect::<Result<_, _>>()?,
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
            let mut document: ReaderDocument = load_paste_in(connection, document_id)?
                .ok_or(StorageError::InvalidDocument)?
                .into();
            if kind.as_deref() == Some("txt") {
                let chapter = &document.chapters[0];
                let ruby =
                    crate::txt::load_source_ruby_in(connection, chapter.chapter_id, &chapter.text)?;
                if !ruby.is_empty() {
                    document.source_ruby.push(project_ruby(chapter, &ruby)?);
                }
            }
            document
        }
        _ => return Err(StorageError::InvalidDocument),
    };
    document
        .validate()
        .map_err(|_| StorageError::InvalidDocument)?;
    Ok(Some(document))
}

fn project_ruby(
    chapter: &yomoku_core::document::ChapterText,
    ruby: &[yomoku_import::txt::SourceRuby],
) -> Result<ReaderChapterRuby, StorageError> {
    let ruby = ruby
        .iter()
        .map(|item| {
            Ok(ReaderRuby {
                range: TextRange {
                    start: TextOffset::at(&chapter.text, item.range.start)?,
                    end: TextOffset::at(&chapter.text, item.range.end)?,
                },
                reading: item.reading.clone(),
            })
        })
        .collect::<Result<_, yomoku_core::document::InvalidTextValue>>()
        .map_err(|_| StorageError::InvalidDocument)?;
    Ok(ReaderChapterRuby {
        chapter_id: chapter.chapter_id,
        ruby,
    })
}
