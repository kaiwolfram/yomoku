//! Native completion ownership across reservation and retries.

use std::sync::Arc;

use super::{EffectivePageCommand, EffectivePageIdentity, EffectiveReaderSession};
use crate::{
    completion::{AnalyzedCompletionOutcome, PageCompletion, PageCompletionIntent},
    document::ReaderDocument,
    effective_reader::EffectivePageCompletion,
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    reader::ReaderAnalysis,
    storage::DocumentError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Prepare {
        intent: PageCompletionIntent,
        command: EffectivePageCommand,
    },
    Commit(Arc<EffectivePageCompletion>),
}

/// Rust-only callback token. An exact-payload retry gets a fresh attempt; an old
/// callback cannot consume it. Full keys/reservations never become shell values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveCompletionRequest {
    attempt: u64,
    action: Action,
}

impl EffectiveCompletionRequest {
    pub fn operation(&self) -> Operation {
        match &self.action {
            Action::Prepare { intent, command } => Operation::PrepareCompletion {
                intent: *intent,
                command: command.clone(),
            },
            Action::Commit(prepared) => Operation::CompletePage(Arc::clone(prepared)),
        }
    }
}

/// Native presentation facts for the app's existing busy/error projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveCompletionState {
    /// Includes preparation, commit and a failed exact retry.
    pub pending: bool,
    pub busy: bool,
    pub error: Option<DocumentError>,
}

/// Only a consumed successful commit may drive position/finish notices.
/// The app uses Recorded/AlreadyRecorded and the frozen command ID as before.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveCompletionProgress {
    Prepared,
    Completed {
        completion: PageCompletion,
        document: Box<ReaderDocument>,
        outcome: AnalyzedCompletionOutcome,
    },
    Failed(DocumentError),
}

#[derive(Default)]
pub(super) struct CompletionState {
    attempt: u64,
    phase: CompletionPhase,
}

#[derive(Default)]
enum CompletionPhase {
    #[default]
    Idle,
    Dispatchable(Action),
    InFlight(EffectiveCompletionRequest),
    Failed {
        action: Action,
        error: DocumentError,
    },
}

impl EffectiveReaderSession {
    pub fn completion_state(&self) -> EffectiveCompletionState {
        EffectiveCompletionState {
            pending: !matches!(self.completion.phase, CompletionPhase::Idle),
            busy: matches!(self.completion.phase, CompletionPhase::InFlight(_)),
            error: match self.completion.phase {
                CompletionPhase::Failed { error, .. } => Some(error),
                _ => None,
            },
        }
    }

    /// Freeze the accepted page identity and immediately retire shell callbacks.
    /// The app checks its current position/preset and shared FIFO/lifecycle gates;
    /// storage revalidates the full rendered partition in its transaction.
    pub fn begin_completion(
        &mut self,
        identity: EffectivePageIdentity,
        intent: PageCompletionIntent,
    ) -> bool {
        if self.completion_state().pending
            || self.words.blocks_completion()
            || self.status_state().pending
            || intent.document_id != identity.base.document_id
            || intent.page != identity.base.page
            || identity.base.analysis != ReaderAnalysis::Available(intent.analysis_id)
            || intent.completed_at_unix_ms < 0
            || intent.departing_position.source_id != intent.page.source_id
            || intent.departing_position.chapter_id != intent.page.chapter_id
            || intent.departing_position.offset < intent.page.range.start
            || intent.departing_position.offset >= intent.page.range.end
        {
            return false;
        }
        let Some(command) = self.freeze_page(identity) else {
            return false;
        };
        self.completion.phase = CompletionPhase::Dispatchable(Action::Prepare { intent, command });
        self.invalidate();
        true
    }

    /// Explicit retry uses this same method; a commit retry shares the exact
    /// durable reservation, never prepares
    /// another one. The app must not auto-retry a Failed progress result.
    pub fn dispatch_completion(&mut self) -> Option<EffectiveCompletionRequest> {
        let action = match &self.completion.phase {
            CompletionPhase::Dispatchable(action) | CompletionPhase::Failed { action, .. } => {
                action
            }
            CompletionPhase::Idle | CompletionPhase::InFlight(_) => return None,
        };
        self.completion.attempt = self.completion.attempt.checked_add(1)?;
        let request = EffectiveCompletionRequest {
            attempt: self.completion.attempt,
            action: action.clone(),
        };
        self.completion.phase = CompletionPhase::InFlight(request.clone());
        Some(request)
    }

    /// Invalidation/refresh never abandons accepted work. There is deliberately
    /// no Cancel: a failed commit may already have recorded history before its
    /// follow-up read failed, and must finish through an exact retry.
    pub fn finish_completion_request(
        &mut self,
        request: &EffectiveCompletionRequest,
        result: Result<Output, DocumentError>,
    ) -> Option<EffectiveCompletionProgress> {
        if !matches!(&self.completion.phase, CompletionPhase::InFlight(current) if current == request)
        {
            return None;
        }
        let progress = match (&request.action, result) {
            (Action::Prepare { intent, command }, Ok(Output::CompletionPrepared(prepared)))
                if matches_preparation(*intent, command, &prepared) =>
            {
                self.completion.phase = CompletionPhase::Dispatchable(Action::Commit(prepared));
                EffectiveCompletionProgress::Prepared
            }
            (Action::Commit(prepared), Ok(Output::PageCompleted(document, outcome)))
                if document.document_id == prepared.completion().document_id
                    && document.validate().is_ok()
                    && document
                        .chapter(prepared.completion().page.chapter_id)
                        .and_then(|chapter| prepared.completion().page.slice(chapter))
                        .is_ok() =>
            {
                self.completion.phase = CompletionPhase::Idle;
                EffectiveCompletionProgress::Completed {
                    completion: prepared.completion(),
                    document: Box::new(document),
                    outcome,
                }
            }
            (_, Err(error)) => EffectiveCompletionProgress::Failed(error),
            _ => EffectiveCompletionProgress::Failed(DocumentError::InvalidDocument),
        };
        if let EffectiveCompletionProgress::Failed(error) = progress {
            self.completion.phase = CompletionPhase::Failed {
                action: request.action.clone(),
                error,
            };
        }
        Some(progress)
    }
}

fn matches_preparation(
    intent: PageCompletionIntent,
    command: &EffectivePageCommand,
    prepared: &EffectivePageCompletion,
) -> bool {
    let completion = prepared.completion();
    prepared.key() == command.key()
        && completion.document_id == intent.document_id
        && completion.page == intent.page
        && completion.page_length == intent.page_length
        && completion.departing_position == intent.departing_position
        && completion.completed_at_unix_ms == intent.completed_at_unix_ms
        && prepared.key().segmentation.original_analysis_id == intent.analysis_id
}
