//! Rust-only effective reader effects, consumed by the session's SQLite FIFO.
//! Full chapter provenance and durable command payloads never cross the shell.

use std::sync::Arc;

use crate::{
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{ReaderDocument, VocabularyId},
    effective_reader::{
        EffectivePageCompletion, EffectiveReaderTokenSnapshot, EffectiveReaderWordDetails,
    },
    effective_session::{
        EffectivePageCommand, EffectivePageRequest, EffectiveReaderSelection,
        EffectiveSelectionCommand,
    },
    reader::ReaderVocabulary,
    vocabulary::VocabularyStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveStorageOperation {
    LoadTokens(EffectivePageRequest),
    LoadWord(EffectiveSelectionCommand),
    SetStatus {
        command: EffectiveSelectionCommand,
        vocabulary_id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    },

    /// Freeze the accepted key before final time drain; reserve only afterward.
    PrepareCompletion {
        intent: PageCompletionIntent,
        command: EffectivePageCommand,
    },
    /// Share one authenticated reservation across exact retries; never replay
    /// unfinished reservations automatically after process restart.
    CompletePage(Arc<EffectivePageCompletion>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveStorageResult {
    TokensLoaded(EffectivePageRequest, EffectiveReaderTokenSnapshot),
    WordLoaded(EffectiveReaderSelection, Box<EffectiveReaderWordDetails>),
    StatusChanged(EffectiveReaderSelection, Option<ReaderVocabulary>),

    CompletionPrepared(Arc<EffectivePageCompletion>),
    PageCompleted(ReaderDocument, AnalyzedCompletionOutcome),
}
