use std::{collections::BTreeSet, error::Error, fmt};

use rusqlite::TransactionBehavior;
use yomoku_core::{
    analysis::{ChapterAnalysis, SplitMode},
    document::{AnalysisId, ChapterText, PasteText, PastedDocument},
    vocabulary::Surface,
};

use crate::{
    analysis::store_chapter_analysis_in, documents::create_paste_in,
    vocabulary::register_vocabulary_in, Storage, StorageError,
};

#[derive(Debug)]
pub struct AnalyzedPaste {
    pub document: PastedDocument,
    pub analysis_id: AnalysisId,
    pub registered_surfaces: usize,
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
    /// Allocate canonical source/chapter IDs, analyze, then commit text, a Mode B
    /// snapshot and unique Japanese registrations together. Run on an I/O worker.
    /// The callback must reuse an already loaded analyzer; it runs while holding
    /// the write transaction and must not perform database work or publish IDs.
    /// Any error rolls back the whole import, including newly allocated IDs.
    /// This repository entry point does not deduplicate documents or start reading.
    pub fn create_analyzed_paste<E>(
        &mut self,
        text: &PasteText,
        registered_at_unix_ms: i64,
        analyze: impl FnOnce(&ChapterText) -> Result<ChapterAnalysis, E>,
    ) -> Result<AnalyzedPaste, PasteImportError<E>> {
        if registered_at_unix_ms < 0 {
            return Err(StorageError::InvalidVocabulary.into());
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(StorageError::from)?;
        let document = create_paste_in(&transaction, text)?;
        let analysis = analyze(&document.chapter).map_err(PasteImportError::Analysis)?;
        if analysis.mode != SplitMode::B {
            return Err(StorageError::InvalidAnalysis.into());
        }
        let analysis_id =
            store_chapter_analysis_in(&transaction, document.chapter.chapter_id, &analysis)?;
        let surfaces = analysis
            .tokens
            .iter()
            .filter(|token| token.is_japanese)
            .map(|token| Surface::new(&token.surface))
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|_| StorageError::InvalidAnalysis)?;
        let registered_surfaces =
            register_vocabulary_in(&transaction, &surfaces, registered_at_unix_ms)?;
        transaction.commit().map_err(StorageError::from)?;
        Ok(AnalyzedPaste {
            document,
            analysis_id,
            registered_surfaces,
        })
    }
}
