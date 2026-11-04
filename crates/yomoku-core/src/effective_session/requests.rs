//! Capture native callback ownership once for every effective-reader operation.

use crux_core::Command;

use super::{
    EffectiveCompletionProgress, EffectiveCompletionRequest, EffectivePageRequest,
    EffectiveReaderSession, EffectiveStatusRequest, EffectiveWordRequest,
};
use crate::{
    app::Effect,
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    storage::{DocumentError, StorageOperation, StorageResult},
};

/// Rust-only work accepted by the retained session. Full keys and exact retry
/// payloads stay native; the shell must never reconstruct a callback token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveReaderRequest {
    Tokens(EffectivePageRequest),
    Word(EffectiveWordRequest),
    Status(EffectiveStatusRequest),
    Completion(EffectiveCompletionRequest),
}

/// A consumed callback, including failures. None from `finish_request` means a
/// stale/duplicate attempt and must not clear the app's newer busy obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveReaderProgress {
    TokensLoaded,
    TokensFailed(DocumentError),
    WordFinished,
    StatusFinished,
    Completion(EffectiveCompletionProgress),
}

impl EffectiveReaderRequest {
    pub fn operation(&self) -> StorageOperation {
        StorageOperation::Effective(match self {
            Self::Tokens(request) => Operation::LoadTokens(*request),
            Self::Word(request) => request.operation(),
            Self::Status(request) => request.operation(),
            Self::Completion(request) => request.operation(),
        })
    }

    /// Send through the existing Crux storage effect/FIFO, capturing this exact
    /// request in the native callback. The app still applies shared clock,
    /// lifecycle and busy gates BEFORE accepting work in the retained session.
    /// Completion preparation must wait for both final checkpoint slots to drain.
    pub fn command<Event: Send + 'static>(
        self,
        finished: impl FnOnce(Self, StorageResult) -> Event + Send + 'static,
    ) -> Command<Effect, Event> {
        Command::request_from_shell(self.operation())
            .then_send(move |result| finished(self, result))
    }
}

impl EffectiveReaderSession {
    /// Route the ordinary FIFO error envelope as well as effective results.
    /// Each lifecycle validates its own result shape and attempt, preserving
    /// frozen write retries on failures. No retry or follow-up is dispatched
    /// here: the app decides what to do with consumed completion progress.
    pub fn finish_request(
        &mut self,
        request: &EffectiveReaderRequest,
        result: StorageResult,
    ) -> Option<EffectiveReaderProgress> {
        let result = match result {
            StorageResult::Effective(output) => Ok(output),
            StorageResult::Failed(error) => Err(error),
            _ => Err(DocumentError::InvalidDocument),
        };
        match request {
            EffectiveReaderRequest::Tokens(request) => {
                let error = match result {
                    Ok(Output::TokensLoaded(returned, snapshot)) if returned == *request => {
                        if self.accept(*request, snapshot) {
                            return Some(EffectiveReaderProgress::TokensLoaded);
                        }
                        DocumentError::InvalidDocument
                    }
                    Err(error) => error,
                    _ => DocumentError::InvalidDocument,
                };
                // Use the captured attempt, never the returned request. A bad
                // response consumes its own read; it cannot consume a newer one.
                self.fail(*request)
                    .then_some(EffectiveReaderProgress::TokensFailed(error))
            }
            EffectiveReaderRequest::Word(request) => self
                .finish_word_request(request, result)
                .then_some(EffectiveReaderProgress::WordFinished),
            EffectiveReaderRequest::Status(request) => self
                .finish_status_request(request, result)
                .then_some(EffectiveReaderProgress::StatusFinished),
            EffectiveReaderRequest::Completion(request) => self
                .finish_completion_request(request, result)
                .map(EffectiveReaderProgress::Completion),
        }
    }
}
