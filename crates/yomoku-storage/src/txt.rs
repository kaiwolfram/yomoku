//! Atomic original-file identity, canonical TXT content and source ruby.

use std::{error::Error, fmt};

use rusqlite::{OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, DocumentId, PastedDocument},
};
use yomoku_import::txt::{self, SourceRuby, TextEncoding, TxtError};

use crate::{
    documents::{create_single_text_in, load_paste_in},
    imports::analyze_document_in,
    AnalyzedPaste, PasteImportError, Storage, StorageError,
};

#[derive(Debug)]
pub enum TxtImportOutcome {
    Created(AnalyzedPaste),
    Existing(PastedDocument),
}

#[derive(Debug)]
pub enum TxtImportError<E> {
    Parse(TxtError),
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for TxtImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => error.fmt(f),
            Self::Analysis(error) => write!(f, "TXT analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for TxtImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for TxtImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

/// Ruby anchors refer only to the saved canonical chapter's UTF-8 bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredTxtSource {
    pub original_bytes: Vec<u8>,
    pub encoding: TextEncoding,
    pub ruby: Vec<SourceRuby>,
}

impl Storage {
    /// Read-only worker preflight: exact live duplicates need no decoder/analyzer.
    /// New files still require the authoritative lookup in the import transaction.
    pub fn find_imported_txt(
        &mut self,
        original_bytes: &[u8],
    ) -> Result<Option<PastedDocument>, StorageError> {
        let digest = Sha256::digest(original_bytes);
        let transaction = self.connection.transaction()?;
        let document = find_imported_in(&transaction, original_bytes, digest.as_slice())?;
        transaction.commit()?;
        Ok(document)
    }

    /// Hash original bytes before decoding; existing live files bypass parsing,
    /// analysis and registration, preserving their title, encoding and progress.
    /// For a new file, filename is its literal display name (not a URI/path).
    /// Run on a worker with a ready analyzer. No provisional IDs may escape the
    /// callback; it must not perform database work. Errors discard this file only.
    pub fn import_analyzed_txt<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<TxtImportOutcome, TxtImportError<E>> {
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let digest = Sha256::digest(original_bytes);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let outcome = if let Some(document) =
            find_imported_in(&transaction, original_bytes, digest.as_slice())?
        {
            TxtImportOutcome::Existing(document)
        } else {
            if filename.trim().is_empty() {
                return Err(StorageError::InvalidDocument.into());
            }
            let parsed = txt::parse(original_bytes, selection).map_err(TxtImportError::Parse)?;
            validate_ruby(&parsed.text, &parsed.ruby)?;
            let document = create_single_text_in(&transaction, "txt", filename, &parsed.text)?;
            transaction.execute(
                "INSERT INTO imported_files (document_id, sha256, original_bytes, text_encoding)
                 VALUES (?1, ?2, ?3, ?4)",
                (i64::from(document.document_id), digest.as_slice(), original_bytes, encoding_name(parsed.encoding)),
            ).map_err(StorageError::from)?;
            for (index, ruby) in parsed.ruby.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO source_ruby (chapter_id, ruby_index, start_byte, end_byte, reading)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    (i64::from(document.chapter.chapter_id), to_i64(index)?,
                     to_i64(ruby.range.start)?, to_i64(ruby.range.end)?, &ruby.reading),
                ).map_err(StorageError::from)?;
            }
            let imported =
                analyze_document_in(&transaction, document, registered_at_unix_ms, analyze)
                    .map_err(|error| match error {
                        PasteImportError::Analysis(error) => TxtImportError::Analysis(error),
                        PasteImportError::Storage(error) => TxtImportError::Storage(error),
                    })?;
            TxtImportOutcome::Created(imported)
        };
        transaction.commit().map_err(StorageError::from)?;
        Ok(outcome)
    }

    /// Read original bytes, selected encoding and canonical ruby without parsing
    /// or loading the analyzer. None means no live TXT document with this ID.
    pub fn load_txt_source(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<StoredTxtSource>, StorageError> {
        let transaction = self.connection.transaction()?;
        let kind: Option<String> = transaction.query_row(
            "SELECT s.kind FROM documents d JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
            [i64::from(document_id)], |row| row.get(0),
        ).optional()?;
        if kind.as_deref() != Some("txt") {
            return Ok(None);
        }
        let source = transaction
            .query_row(
                "SELECT f.original_bytes, f.text_encoding, f.sha256 FROM imported_files f
             JOIN documents d ON d.id = f.document_id JOIN sources s ON s.id = d.source_id
             WHERE d.id = ?1 AND s.kind = 'txt'",
                [i64::from(document_id)],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()?;
        let (original_bytes, encoding, digest) = source.ok_or(StorageError::InvalidDocument)?;
        if Sha256::digest(&original_bytes).as_slice() != digest {
            return Err(StorageError::InvalidDocument);
        }
        let encoding = match encoding.as_str() {
            "utf8" => TextEncoding::Utf8,
            "shift_jis" => TextEncoding::ShiftJis,
            _ => return Err(StorageError::InvalidDocument),
        };
        let document =
            load_paste_in(&transaction, document_id)?.ok_or(StorageError::InvalidDocument)?;
        let ruby = {
            let mut statement = transaction.prepare(
                "SELECT ruby_index, start_byte, end_byte, reading FROM source_ruby
                 WHERE chapter_id = ?1 ORDER BY ruby_index",
            )?;
            let rows = statement.query_map([i64::from(document.chapter.chapter_id)], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            rows.enumerate()
                .map(|(index, row)| {
                    let (stored_index, start, end, reading) = row?;
                    if stored_index != to_i64(index)? {
                        return Err(StorageError::InvalidDocument);
                    }
                    Ok(SourceRuby {
                        range: usize::try_from(start).map_err(|_| StorageError::InvalidDocument)?
                            ..usize::try_from(end).map_err(|_| StorageError::InvalidDocument)?,
                        reading,
                    })
                })
                .collect::<Result<Vec<_>, StorageError>>()?
        };
        validate_ruby(&document.chapter.text, &ruby)?;
        transaction.commit()?;
        Ok(Some(StoredTxtSource {
            original_bytes,
            encoding,
            ruby,
        }))
    }
}

fn find_imported_in(
    connection: &rusqlite::Connection,
    original_bytes: &[u8],
    digest: &[u8],
) -> Result<Option<PastedDocument>, StorageError> {
    let existing: Option<(i64, Vec<u8>)> = connection
        .query_row(
            "SELECT document_id, original_bytes FROM imported_files WHERE sha256 = ?1",
            [digest],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((id, stored_bytes)) = existing else {
        return Ok(None);
    };
    // Fail closed on a stale/corrupt key or digest collision.
    if stored_bytes != original_bytes {
        return Err(StorageError::InvalidDocument);
    }
    let id = DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?;
    load_paste_in(connection, id)?.map_or(Err(StorageError::InvalidDocument), |document| {
        Ok(Some(document))
    })
}

fn validate_ruby(text: &str, ruby: &[SourceRuby]) -> Result<(), StorageError> {
    let mut previous_end = 0;
    for item in ruby {
        if item.range.start < previous_end
            || item.range.is_empty()
            || text.get(item.range.clone()).is_none()
            || item.reading.is_empty()
        {
            return Err(StorageError::InvalidDocument);
        }
        previous_end = item.range.end;
    }
    Ok(())
}

fn to_i64(value: usize) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::InvalidDocument)
}

fn encoding_name(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::ShiftJis => "shift_jis",
    }
}
