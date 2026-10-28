//! Rust-only effective reader effects, consumed by the session's SQLite FIFO.
//! Full chapter provenance and durable command payloads never cross the shell.

use std::sync::Arc;

use crate::{
    analysis::OverrideScope,
    completion::{AnalyzedCompletionOutcome, PageCompletionIntent},
    document::{OverrideId, ReaderDocument, TextAnchor, VocabularyId},
    effective_overrides::{EffectiveOverrideContext, EffectiveOverrideFailure},
    effective_reader::{
        EffectivePageCompletion, EffectiveReaderTokenSnapshot, EffectiveReaderWordDetails,
    },
    effective_session::{
        EffectivePageCommand, EffectivePageRequest, EffectiveReaderSelection,
        EffectiveSelectionCommand,
    },
    reader::{ReaderCustomEntry, ReaderVocabulary},
    vocabulary::{Surface, VocabularyStatus},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveStorageOperation {
    LoadTokens(EffectivePageRequest),
    LoadOverrideContext {
        command: EffectivePageCommand,
        anchor: TextAnchor,
    },
    SaveOverride {
        context: Arc<EffectiveOverrideContext>,
        scope: OverrideScope,
        parts: Vec<Surface>,
        registered_at_unix_ms: i64,
    },
    ReverseOverride {
        context: Arc<EffectiveOverrideContext>,
        id: OverrideId,
    },
    LoadWord(EffectiveSelectionCommand),
    SetStatus {
        command: EffectiveSelectionCommand,
        vocabulary_id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    },
    SaveCustomEntry {
        command: EffectiveSelectionCommand,
        english_meaning: String,
        reading: String,
    },
    DeleteCustomEntry(EffectiveSelectionCommand),
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
    OverrideContextLoaded(Arc<EffectiveOverrideContext>),
    OverrideSaved { id: OverrideId, registered: usize },
    OverrideReversed(OverrideId),
    OverrideFailed(EffectiveOverrideFailure),
    WordLoaded(EffectiveReaderSelection, Box<EffectiveReaderWordDetails>),
    StatusChanged(EffectiveReaderSelection, Option<ReaderVocabulary>),
    CustomEntryChanged(EffectiveReaderSelection, Option<ReaderCustomEntry>),
    CompletionPrepared(Arc<EffectivePageCompletion>),
    PageCompleted(ReaderDocument, AnalyzedCompletionOutcome),
}
