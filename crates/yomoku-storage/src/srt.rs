//! Atomic original SRT bytes, canonical cue anchors and analyzed registration.

use std::{error::Error, fmt, ops::Range};

use rusqlite::{OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::ChapterAnalysis,
    document::{ChapterText, DocumentId, PastedDocument},
};
use yomoku_import::{
    srt::{self, EmptySrt},
    txt::{decode, TextEncoding, TxtError},
};

use crate::{
    documents::{create_single_text_in, load_paste_in},
    files::{encoding_name, find_imported_in},
    imports::store_analyzed_document_in,
    AnalyzedPaste, ImportCancellation, Storage, StorageError,
};

#[derive(Debug)]
pub enum SrtImportOutcome {
    Created(AnalyzedPaste),
    Existing(PastedDocument),
}

#[derive(Debug)]
pub enum SrtImportError<E> {
    /// Uses the shared strict text decoder; no Aozora transformation is applied.
    Decode(TxtError),
    Empty(EmptySrt),
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for SrtImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "SRT decoding failed: {error}"),
            Self::Empty(error) => error.fmt(f),
            Self::Analysis(error) => write!(f, "SRT analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for SrtImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Empty(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for SrtImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredSrtSource {
    pub original_bytes: Vec<u8>,
    pub encoding: TextEncoding,
    /// Canonical chapter-local UTF-8 bytes; separators are outside each range.
    pub cues: Vec<Range<usize>>,
}

impl Storage {
    /// Hash before decoding. Exact live duplicates retain their interpretation,
    /// title, progress and statuses without invoking the analyzer. A new SRT
    /// uses its literal display filename as title and one canonical chapter.
    /// The ready worker callback must not publish provisional IDs or do DB I/O.
    pub fn import_analyzed_srt<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<SrtImportOutcome, SrtImportError<E>> {
        self.import_analyzed_srt_cancellable(
            original_bytes,
            filename,
            selection,
            registered_at_unix_ms,
            &ImportCancellation::default(),
            analyze,
        )
        .map(|outcome| outcome.expect("unshared import cannot be cancelled"))
    }

    /// Share a fresh handle before worker submission and request cancellation
    /// outside that worker. None discards this file, including allocated IDs;
    /// Some retains a winning commit. Prior files are never affected.
    pub fn import_analyzed_srt_cancellable<E>(
        &mut self,
        original_bytes: &[u8],
        filename: &str,
        selection: Option<TextEncoding>,
        registered_at_unix_ms: i64,
        cancellation: &ImportCancellation,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<Option<SrtImportOutcome>, SrtImportError<E>> {
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
        let outcome = if let Some(document) =
            find_imported_in(&transaction, original_bytes, digest.as_slice())?
        {
            SrtImportOutcome::Existing(document)
        } else {
            if filename.trim().is_empty() {
                return Err(StorageError::InvalidDocument.into());
            }
            let (encoding, decoded) =
                decode(original_bytes, selection).map_err(SrtImportError::Decode)?;
            let parsed = srt::parse(&decoded).map_err(SrtImportError::Empty)?;
            validate_cues(&parsed.text, &parsed.cues)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let document = create_single_text_in(&transaction, "srt", filename, &parsed.text)?;
            transaction.execute(
                "INSERT INTO imported_files (document_id, sha256, original_bytes, text_encoding)
                 VALUES (?1, ?2, ?3, ?4)",
                (i64::from(document.document_id), digest.as_slice(), original_bytes, encoding_name(encoding)),
            ).map_err(StorageError::from)?;
            for (index, cue) in parsed.cues.iter().enumerate() {
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                transaction
                    .execute(
                        "INSERT INTO subtitle_cues (chapter_id, cue_index, start_byte, end_byte)
                     VALUES (?1, ?2, ?3, ?4)",
                        (
                            i64::from(document.chapter.chapter_id),
                            to_i64(index)?,
                            to_i64(cue.start)?,
                            to_i64(cue.end)?,
                        ),
                    )
                    .map_err(StorageError::from)?;
            }
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let analysis = analyze(&document.chapter).map_err(SrtImportError::Analysis)?;
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            SrtImportOutcome::Created(store_analyzed_document_in(
                &transaction,
                document,
                registered_at_unix_ms,
                &analysis,
            )?)
        };
        cancellation.commit(|| {
            transaction.commit().map_err(StorageError::from)?;
            Ok(outcome)
        })
    }

    /// Read original bytes/encoding/cues atomically without decoding or analysis.
    /// None means no live SRT with this ID; inconsistent saved data fails closed.
    pub fn load_srt_source(
        &mut self,
        document_id: DocumentId,
    ) -> Result<Option<StoredSrtSource>, StorageError> {
        let transaction = self.connection.transaction()?;
        let kind: Option<String> = transaction.query_row(
            "SELECT s.kind FROM documents d JOIN sources s ON s.id = d.source_id WHERE d.id = ?1",
            [i64::from(document_id)], |row| row.get(0),
        ).optional()?;
        if kind.as_deref() != Some("srt") {
            return Ok(None);
        }
        let source = transaction.query_row(
            "SELECT original_bytes, text_encoding, sha256 FROM imported_files WHERE document_id = ?1",
            [i64::from(document_id)],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?, row.get::<_, Vec<u8>>(2)?)),
        ).optional()?;
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
        let cues = {
            let mut statement = transaction.prepare(
                "SELECT cue_index, start_byte, end_byte FROM subtitle_cues
                 WHERE chapter_id = ?1 ORDER BY cue_index",
            )?;
            let rows = statement.query_map([i64::from(document.chapter.chapter_id)], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            rows.enumerate()
                .map(|(index, row)| {
                    let (stored_index, start, end) = row?;
                    if stored_index != to_i64(index)? {
                        return Err(StorageError::InvalidDocument);
                    }
                    Ok(
                        usize::try_from(start).map_err(|_| StorageError::InvalidDocument)?
                            ..usize::try_from(end).map_err(|_| StorageError::InvalidDocument)?,
                    )
                })
                .collect::<Result<Vec<_>, StorageError>>()?
        };
        validate_cues(&document.chapter.text, &cues)?;
        transaction.commit()?;
        Ok(Some(StoredSrtSource {
            original_bytes,
            encoding,
            cues,
        }))
    }
}

fn validate_cues(text: &str, cues: &[Range<usize>]) -> Result<(), StorageError> {
    let mut end = 0;
    for (index, cue) in cues.iter().enumerate() {
        let gap = if index == 0 { "" } else { "\n\n" };
        if cue.is_empty()
            || text.get(end..cue.start) != Some(gap)
            || text
                .get(cue.clone())
                .is_none_or(|text| text.trim().is_empty())
        {
            return Err(StorageError::InvalidDocument);
        }
        end = cue.end;
    }
    if cues.is_empty() || end != text.len() {
        return Err(StorageError::InvalidDocument);
    }
    Ok(())
}

fn to_i64(value: usize) -> Result<i64, StorageError> {
    i64::try_from(value).map_err(|_| StorageError::InvalidDocument)
}
