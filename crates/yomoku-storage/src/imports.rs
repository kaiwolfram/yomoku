use std::{collections::BTreeSet, error::Error, fmt};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::{AnalysisId, ChapterId, ChapterText, DocumentId, PasteText, PastedDocument},
    vocabulary::Surface,
};

use crate::{
    analysis::store_chapter_analysis_in,
    documents::{create_paste_in, load_paste_in},
    vocabulary::register_vocabulary_in,
    Storage, StorageError,
};

#[derive(Debug)]
pub struct AnalyzedPaste {
    pub document: PastedDocument,
    pub analysis_id: AnalysisId,
    pub registered_surfaces: usize,
}

#[derive(Debug)]
pub enum PasteImportOutcome {
    Created(AnalyzedPaste),
    /// Reference the committed live document without analysis, registration or
    /// changes to title/progress. This may predate analyzed imports entirely.
    Existing(PastedDocument),
}

/// Keep analyzer diagnostics typed without coupling SQLite to a particular engine.
#[derive(Debug)]
pub enum PasteImportError<E> {
    Analysis(E),
    Storage(StorageError),
}

impl<E: fmt::Display> fmt::Display for PasteImportError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Analysis(error) => write!(f, "paste analysis failed: {error}"),
            Self::Storage(error) => error.fmt(f),
        }
    }
}

impl<E: Error + 'static> Error for PasteImportError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Analysis(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

impl<E> From<StorageError> for PasteImportError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl Storage {
    /// Atomically deduplicate exact pasted UTF-8 bytes before analyzing a new
    /// document. The callback is never invoked for an existing document.
    /// Hashes are committed atomically with every new paste.
    /// Run on a worker with a ready analyzer.
    pub fn import_analyzed_paste<E>(
        &mut self,
        text: &PasteText,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<PasteImportOutcome, PasteImportError<E>> {
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let digest = Sha256::digest(text.as_str().as_bytes());
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let existing: Option<i64> = transaction
            .query_row(
                "SELECT document_id FROM paste_content_hashes WHERE sha256 = ?1
                 ORDER BY document_id LIMIT 1",
                [digest.as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(StorageError::from)?;
        let outcome = if let Some(id) = existing {
            let id = DocumentId::try_from(id).map_err(|_| StorageError::InvalidDocument)?;
            let document = load_paste_in(&transaction, id)?.ok_or(StorageError::InvalidDocument)?;
            // A stale/corrupt digest (or collision) must never reference unrelated text.
            if document.chapter.text != text.as_str() {
                return Err(StorageError::InvalidDocument.into());
            }
            PasteImportOutcome::Existing(document)
        } else {
            let imported =
                create_analyzed_paste_in(&transaction, text, registered_at_unix_ms, analyze)?;
            PasteImportOutcome::Created(imported)
        };
        transaction.commit().map_err(StorageError::from)?;
        Ok(outcome)
    }
}

fn create_analyzed_paste_in<E>(
    connection: &Connection,
    text: &PasteText,
    registered_at_unix_ms: i64,
    analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
) -> Result<AnalyzedPaste, PasteImportError<E>> {
    let document = create_paste_in(connection, text)?;
    analyze_document_in(connection, document, registered_at_unix_ms, analyze)
}

pub(crate) fn analyze_document_in<E>(
    connection: &Connection,
    document: PastedDocument,
    registered_at_unix_ms: i64,
    analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
) -> Result<AnalyzedPaste, PasteImportError<E>> {
    let analysis = analyze(&document.chapter).map_err(PasteImportError::Analysis)?;
    store_analyzed_document_in(connection, document, registered_at_unix_ms, &analysis)
        .map_err(PasteImportError::Storage)
}

pub(crate) fn store_analyzed_document_in(
    connection: &Connection,
    document: PastedDocument,
    registered_at_unix_ms: i64,
    analysis: &ChapterAnalysis,
) -> Result<AnalyzedPaste, StorageError> {
    let (analysis_id, registered_surfaces) = store_analyzed_chapter_in(
        connection,
        document.chapter.chapter_id,
        registered_at_unix_ms,
        analysis,
    )?;
    Ok(AnalyzedPaste {
        document,
        analysis_id,
        registered_surfaces,
    })
}

pub(crate) fn store_analyzed_chapter_in(
    connection: &Connection,
    chapter_id: ChapterId,
    registered_at_unix_ms: i64,
    analysis: &ChapterAnalysis,
) -> Result<(AnalysisId, usize), StorageError> {
    if analysis.mode != SplitMode::C {
        return Err(StorageError::InvalidAnalysis);
    }
    let analysis_id = store_chapter_analysis_in(connection, chapter_id, analysis)?;
    let surfaces = analysis
        .tokens
        .iter()
        .filter(|token| token.is_japanese)
        .map(|token| Surface::new(&token.surface))
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|_| StorageError::InvalidAnalysis)?;
    let registered_surfaces = register_vocabulary_in(connection, &surfaces, registered_at_unix_ms)?;
    Ok((analysis_id, registered_surfaces))
}
