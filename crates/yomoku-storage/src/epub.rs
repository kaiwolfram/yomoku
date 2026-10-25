//! Atomic EPUB container, ordered canonical chapters, assets and Mode B snapshots.

use std::{collections::BTreeMap, error::Error, fmt, ops::Range};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{AnalysisId, ChapterId, ChapterText, DocumentId, SourceId, TextOffset},
    pagination::ReadingPosition,
};
use yomoku_import::{
    epub::{self, EpubChapter, EpubError, EpubImage, EpubSpan, InlineImage, SpanKind},
    txt::SourceRuby,
};

use crate::{
    files::find_imported_id_in, imports::store_analyzed_chapter_in, txt::validate_ruby,
    ImportCancellation, Storage, StorageError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredEpubChapter {
    pub chapter: ChapterText,
    pub path: String,
    pub ruby: Vec<SourceRuby>,
    pub spans: Vec<EpubSpan>,
    pub images: Vec<InlineImage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredEpubDocument {
    pub document_id: DocumentId,
    pub source_id: SourceId,
    pub title: String,
    pub filename: String,
    pub finished_at_least_once: bool,
    pub reading_position: Option<ReadingPosition>,
    pub original_bytes: Vec<u8>,
    pub metadata_title: Option<String>,
    pub cover: Option<String>,
    pub images: BTreeMap<String, EpubImage>,
    pub chapters: Vec<StoredEpubChapter>,
}

#[derive(Debug)]
pub struct AnalyzedEpub {
    pub document: StoredEpubDocument,
    /// Same order as document.chapters; each is the original stored Mode B.
    pub analysis_ids: Vec<AnalysisId>,
    pub registered_surfaces: usize,
}

#[derive(Debug)]
pub enum EpubImportOutcome {
    Created(Box<AnalyzedEpub>),
    /// Exact bytes retain the first live interpretation, including TXT/SRT.
    /// Loading the document is separate and never requires analysis or parsing.
    Existing(DocumentId),
}

#[derive(Debug)]
pub enum EpubImportError<E> {
    Parse(EpubError),
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for EpubImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Analysis(error) => write!(f, "EPUB analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for EpubImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for EpubImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl Storage {
    /// Ready-analyzer callback runs once per canonical spine chapter inside one
    /// transaction. It must not publish provisional IDs or perform database I/O.
    /// All chapters, registrations and assets commit together, without encounters.
    pub fn import_analyzed_epub<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        registered_at_unix_ms: i64,
        analyze: impl FnMut(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<EpubImportOutcome, EpubImportError<E>> {
        self.import_analyzed_epub_cancellable(
            original_bytes,
            filename,
            registered_at_unix_ms,
            &ImportCancellation::default(),
            analyze,
        )
        .map(|outcome| outcome.expect("unshared import cannot be cancelled"))
    }

    /// None means the entire file rolled back, including chapter IDs. Cancel
    /// outside the FIFO, off main; the shared final commit gate retains winners.
    pub fn import_analyzed_epub_cancellable<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        registered_at_unix_ms: i64,
        cancellation: &ImportCancellation,
        mut analyze: impl FnMut(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<Option<EpubImportOutcome>, EpubImportError<E>> {
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let digest = Sha256::digest(original_bytes);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        let outcome = if let Some(id) =
            find_imported_id_in(&transaction, original_bytes, digest.as_slice())?
        {
            EpubImportOutcome::Existing(id)
        } else {
            if filename.trim().is_empty() {
                return Err(StorageError::InvalidDocument.into());
            }
            // Always use the bounded parser; no stored metadata can tune limits.
            let parsed = epub::parse(original_bytes).map_err(EpubImportError::Parse)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let title = parsed.title.as_deref().unwrap_or(filename);
            transaction
                .execute(
                    "INSERT INTO sources (kind, original_title) VALUES ('epub', ?1)",
                    [title],
                )
                .map_err(StorageError::from)?;
            let source_id = SourceId::try_from(transaction.last_insert_rowid())
                .map_err(|_| StorageError::InvalidDocument)?;
            transaction
                .execute(
                    "INSERT INTO documents (source_id, title) VALUES (?1, ?2)",
                    (i64::from(source_id), title),
                )
                .map_err(StorageError::from)?;
            let document_id = DocumentId::try_from(transaction.last_insert_rowid())
                .map_err(|_| StorageError::InvalidDocument)?;
            transaction.execute("INSERT INTO imported_files (document_id, sha256, original_bytes, text_encoding) VALUES (?1, ?2, ?3, 'epub')",
                (i64::from(document_id), digest.as_slice(), original_bytes)).map_err(StorageError::from)?;
            transaction.execute("INSERT INTO epub_documents (document_id, filename, metadata_title, cover_path, chapter_count, asset_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (i64::from(document_id), filename, &parsed.title, &parsed.cover, number(parsed.chapters.len())?, number(parsed.images.len())?)).map_err(StorageError::from)?;
            for (path, image) in &parsed.images {
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                transaction.execute("INSERT INTO epub_assets (document_id, path, media_type, bytes, sha256) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (i64::from(document_id), path, &image.media_type, &image.bytes, Sha256::digest(&image.bytes).as_slice())).map_err(StorageError::from)?;
            }
            let mut chapters = Vec::new();
            let mut analysis_ids = Vec::new();
            let mut registered_surfaces = 0;
            for (index, content) in parsed.chapters.into_iter().enumerate() {
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                validate_content(&content)?;
                transaction
                    .execute(
                        "INSERT INTO chapters (source_id, chapter_index) VALUES (?1, ?2)",
                        (i64::from(source_id), number(index)?),
                    )
                    .map_err(StorageError::from)?;
                let chapter_id = ChapterId::try_from(transaction.last_insert_rowid())
                    .map_err(|_| StorageError::InvalidDocument)?;
                transaction
                    .execute(
                        "INSERT INTO chapter_texts (chapter_id, text) VALUES (?1, ?2)",
                        (i64::from(chapter_id), &content.text),
                    )
                    .map_err(StorageError::from)?;
                store_content(&transaction, document_id, chapter_id, &content)?;
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                let chapter = ChapterText {
                    source_id,
                    chapter_id,
                    text: content.text,
                };
                let analysis = analyze(&chapter).map_err(EpubImportError::Analysis)?;
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                let (analysis_id, registered) = store_analyzed_chapter_in(
                    &transaction,
                    chapter_id,
                    registered_at_unix_ms,
                    &analysis,
                )?;
                analysis_ids.push(analysis_id);
                registered_surfaces += registered;
                chapters.push(StoredEpubChapter {
                    chapter,
                    path: content.path,
                    ruby: content.ruby,
                    spans: content.spans,
                    images: content.images,
                });
            }
            EpubImportOutcome::Created(Box::new(AnalyzedEpub {
                document: StoredEpubDocument {
                    document_id,
                    source_id,
                    title: title.to_owned(),
                    filename: filename.to_owned(),
                    finished_at_least_once: false,
                    reading_position: None,
                    original_bytes: original_bytes.to_vec(),
                    metadata_title: parsed.title,
                    cover: parsed.cover,
                    images: parsed.images,
                    chapters,
                },
                analysis_ids,
                registered_surfaces,
            }))
        };
        cancellation.commit(|| {
            transaction.commit().map_err(StorageError::from)?;
            Ok(outcome)
        })
    }

    /// Read a consistent multi-chapter snapshot without ZIP parsing or analysis.
    /// Saved counts/digests/UTF-8 anchors are checked; corruption is never repaired.
    pub fn load_epub_document(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<StoredEpubDocument>, StorageError> {
        let transaction = self.connection.transaction()?;
        let result = load_epub_in(&transaction, document_id)?;
        transaction.commit()?;
        Ok(result)
    }
}

fn store_content(
    connection: &Connection,
    document_id: DocumentId,
    chapter_id: ChapterId,
    content: &EpubChapter,
) -> Result<(), StorageError> {
    let id = i64::from(chapter_id);
    connection.execute("INSERT INTO epub_chapters (chapter_id, document_id, path, text_sha256, ruby_count, span_count, image_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (id, i64::from(document_id), &content.path, Sha256::digest(content.text.as_bytes()).as_slice(), number(content.ruby.len())?, number(content.spans.len())?, number(content.images.len())?))?;
    for (index, ruby) in content.ruby.iter().enumerate() {
        connection.execute("INSERT INTO source_ruby (chapter_id, ruby_index, start_byte, end_byte, reading) VALUES (?1, ?2, ?3, ?4, ?5)",
            (id, number(index)?, number(ruby.range.start)?, number(ruby.range.end)?, &ruby.reading))?;
    }
    for (index, span) in content.spans.iter().enumerate() {
        let (kind, level, target) = match &span.kind {
            SpanKind::Heading(level) => ("heading", Some(i64::from(*level)), None),
            SpanKind::Emphasis => ("emphasis", None, None),
            SpanKind::Strong => ("strong", None, None),
            SpanKind::ExternalLink(target) => ("external", None, Some(target)),
            SpanKind::InertLink(target) => ("inert", None, Some(target)),
        };
        connection.execute("INSERT INTO epub_spans (chapter_id, span_index, start_byte, end_byte, kind, heading_level, target) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (id, number(index)?, number(span.range.start)?, number(span.range.end)?, kind, level, target))?;
    }
    for (index, image) in content.images.iter().enumerate() {
        connection.execute("INSERT INTO epub_inline_images (chapter_id, document_id, image_index, start_byte, end_byte, path, alt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (id, i64::from(document_id), number(index)?, number(image.range.start)?, number(image.range.end)?, &image.path, &image.alt))?;
    }
    Ok(())
}

pub(crate) fn load_epub_in(
    connection: &Connection,
    document_id: DocumentId,
) -> Result<Option<StoredEpubDocument>, StorageError> {
    let header = connection.query_row("SELECT d.source_id, d.title, d.finished_at_least_once, s.kind FROM documents d JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
        [i64::from(document_id)], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?, row.get::<_, String>(3)?))).optional()?;
    let Some((source_id, title, finished_at_least_once, kind)) = header else {
        return Ok(None);
    };
    if kind != "epub" {
        return Ok(None);
    }
    let source_id = SourceId::try_from(source_id).map_err(|_| StorageError::InvalidDocument)?;
    let header = connection.query_row("SELECT f.original_bytes, f.sha256, f.text_encoding, e.filename, e.metadata_title, e.cover_path, e.chapter_count, e.asset_count FROM imported_files f JOIN epub_documents e ON e.document_id = f.document_id WHERE f.document_id = ?1",
        [i64::from(document_id)], |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?, row.get::<_, i64>(6)?, row.get::<_, i64>(7)?))).optional()?.ok_or(StorageError::InvalidDocument)?;
    let (
        original_bytes,
        digest,
        encoding,
        filename,
        metadata_title,
        cover,
        chapter_count,
        asset_count,
    ) = header;
    if encoding != "epub"
        || Sha256::digest(&original_bytes).as_slice() != digest
        || filename.trim().is_empty()
        || metadata_title
            .as_ref()
            .is_some_and(|title| title.trim().is_empty())
    {
        return Err(StorageError::InvalidDocument);
    }
    let mut images = BTreeMap::new();
    let mut statement = connection.prepare("SELECT path, media_type, bytes, sha256 FROM epub_assets WHERE document_id = ?1 ORDER BY path")?;
    let rows = statement.query_map([i64::from(document_id)], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Vec<u8>>(2)?,
            row.get::<_, Vec<u8>>(3)?,
        ))
    })?;
    for row in rows {
        let (path, media_type, bytes, digest) = row?;
        if path.is_empty()
            || !media_type.starts_with("image/")
            || Sha256::digest(&bytes).as_slice() != digest
        {
            return Err(StorageError::InvalidDocument);
        }
        images.insert(path, EpubImage { media_type, bytes });
    }
    if number(images.len())? != asset_count
        || cover
            .as_ref()
            .is_some_and(|path| !images.contains_key(path))
    {
        return Err(StorageError::InvalidDocument);
    }
    // Start from provenance so a missing live chapter/metadata row cannot silently
    // omit a spine item or reinterpret another source's chapter as this book.
    let mut statement = connection.prepare("SELECT c.id, c.chapter_index, t.text, e.path, e.text_sha256, e.ruby_count, e.span_count, e.image_count, e.document_id FROM chapters c LEFT JOIN chapter_texts t ON t.chapter_id = c.id LEFT JOIN epub_chapters e ON e.chapter_id = c.id WHERE c.source_id = ?1 ORDER BY c.chapter_index")?;
    let rows = statement.query_map([i64::from(source_id)], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<Vec<u8>>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<i64>>(6)?,
            row.get::<_, Option<i64>>(7)?,
            row.get::<_, Option<i64>>(8)?,
        ))
    })?;
    let mut chapters = Vec::new();
    for (index, row) in rows.enumerate() {
        let (
            chapter_id,
            stored_index,
            text,
            path,
            digest,
            ruby_count,
            span_count,
            image_count,
            owner,
        ) = row?;
        let text = text.ok_or(StorageError::InvalidDocument)?;
        if stored_index != number(index)?
            || owner != Some(i64::from(document_id))
            || digest.as_deref() != Some(Sha256::digest(text.as_bytes()).as_slice())
        {
            return Err(StorageError::InvalidDocument);
        }
        let chapter_id =
            ChapterId::try_from(chapter_id).map_err(|_| StorageError::InvalidDocument)?;
        let content = read_content(
            connection,
            chapter_id,
            path.ok_or(StorageError::InvalidDocument)?,
            text,
        )?;
        if ruby_count != Some(number(content.ruby.len())?)
            || span_count != Some(number(content.spans.len())?)
            || image_count != Some(number(content.images.len())?)
            || content
                .images
                .iter()
                .any(|image| !images.contains_key(&image.path))
        {
            return Err(StorageError::InvalidDocument);
        }
        validate_content(&content)?;
        chapters.push(StoredEpubChapter {
            chapter: ChapterText {
                source_id,
                chapter_id,
                text: content.text,
            },
            path: content.path,
            ruby: content.ruby,
            spans: content.spans,
            images: content.images,
        });
    }
    if chapters.is_empty() || number(chapters.len())? != chapter_count {
        return Err(StorageError::InvalidDocument);
    }
    let position = connection
        .query_row(
            "SELECT chapter_id, byte_offset FROM reading_positions WHERE document_id = ?1",
            [i64::from(document_id)],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    let reading_position = position
        .map(|(id, offset)| {
            let chapter_id = ChapterId::try_from(id).map_err(|_| StorageError::InvalidDocument)?;
            let chapter = chapters
                .iter()
                .find(|chapter| chapter.chapter.chapter_id == chapter_id)
                .ok_or(StorageError::InvalidDocument)?;
            let position = ReadingPosition {
                source_id,
                chapter_id,
                offset: TextOffset::try_from(offset).map_err(|_| StorageError::InvalidDocument)?,
            };
            position
                .validate(&chapter.chapter)
                .map_err(|_| StorageError::InvalidDocument)?;
            Ok::<_, StorageError>(position)
        })
        .transpose()?;
    Ok(Some(StoredEpubDocument {
        document_id,
        source_id,
        title,
        filename,
        finished_at_least_once,
        reading_position,
        original_bytes,
        metadata_title,
        cover,
        images,
        chapters,
    }))
}

fn read_content(
    connection: &Connection,
    chapter_id: ChapterId,
    path: String,
    text: String,
) -> Result<EpubChapter, StorageError> {
    let id = i64::from(chapter_id);
    let mut content = EpubChapter {
        path,
        text,
        ruby: Vec::new(),
        spans: Vec::new(),
        images: Vec::new(),
    };
    let mut statement = connection.prepare("SELECT ruby_index, start_byte, end_byte, reading FROM source_ruby WHERE chapter_id = ?1 ORDER BY ruby_index")?;
    let rows = statement.query_map([id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for (index, row) in rows.enumerate() {
        let (stored_index, start, end, reading) = row?;
        check_index(index, stored_index)?;
        content.ruby.push(SourceRuby {
            range: range(start, end)?,
            reading,
        });
    }
    let mut statement = connection.prepare("SELECT span_index, start_byte, end_byte, kind, heading_level, target FROM epub_spans WHERE chapter_id = ?1 ORDER BY span_index")?;
    let rows = statement.query_map([id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    for (index, row) in rows.enumerate() {
        let (stored_index, start, end, kind, level, target) = row?;
        check_index(index, stored_index)?;
        let kind = match (kind.as_str(), level, target) {
            ("heading", Some(level @ 1..=6), None) => SpanKind::Heading(level as u8),
            ("emphasis", None, None) => SpanKind::Emphasis,
            ("strong", None, None) => SpanKind::Strong,
            ("external", None, Some(target)) if website(&target) => SpanKind::ExternalLink(target),
            ("inert", None, Some(target)) => SpanKind::InertLink(target),
            _ => return Err(StorageError::InvalidDocument),
        };
        content.spans.push(EpubSpan {
            range: range(start, end)?,
            kind,
        });
    }
    let mut statement = connection.prepare("SELECT image_index, start_byte, end_byte, path, alt FROM epub_inline_images WHERE chapter_id = ?1 ORDER BY image_index")?;
    let rows = statement.query_map([id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    for (index, row) in rows.enumerate() {
        let (stored_index, start, end, path, alt) = row?;
        check_index(index, stored_index)?;
        content.images.push(InlineImage {
            range: range(start, end)?,
            path,
            alt,
        });
    }
    Ok(content)
}

fn validate_content(content: &EpubChapter) -> Result<(), StorageError> {
    if content.path.is_empty() || content.text.trim().is_empty() {
        return Err(StorageError::InvalidDocument);
    }
    validate_ruby(&content.text, &content.ruby)?;
    for span in &content.spans {
        if span.range.is_empty() || content.text.get(span.range.clone()).is_none() {
            return Err(StorageError::InvalidDocument);
        }
    }
    let mut end = 0;
    for image in &content.images {
        if image.range.start < end
            || content.text.get(image.range.clone()) != Some("\u{fffc}")
            || image.path.is_empty()
        {
            return Err(StorageError::InvalidDocument);
        }
        end = image.range.end;
    }
    Ok(())
}

fn number(value: usize) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::InvalidDocument)
}

fn range(start: i64, end: i64) -> Result<Range<usize>, StorageError> {
    Ok(
        usize::try_from(start).map_err(|_| StorageError::InvalidDocument)?
            ..usize::try_from(end).map_err(|_| StorageError::InvalidDocument)?,
    )
}

fn check_index(index: usize, stored: i64) -> Result<(), StorageError> {
    if number(index)? != stored {
        return Err(StorageError::InvalidDocument);
    }
    Ok(())
}

fn website(target: &str) -> bool {
    target.split_once("://").is_some_and(|(scheme, rest)| {
        (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
            && !rest.is_empty()
            && !rest.starts_with('/')
            && !target.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}
