//! Native manual status ownership; shell callbacks carry only compact selections.

use super::{EffectiveReaderSelection, EffectiveReaderSession, EffectiveSelectionCommand};
use crate::{
    document::VocabularyId,
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    storage::DocumentError,
    vocabulary::VocabularyStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveStatusRequest {
    attempt: u64,
    command: EffectiveSelectionCommand,
    vocabulary_id: VocabularyId,
    surface: String,
    status: VocabularyStatus,
    occurred_at_unix_ms: i64,
}

impl EffectiveStatusRequest {
    pub fn operation(&self) -> Operation {
        Operation::SetStatus {
            command: self.command.clone(),
            vocabulary_id: self.vocabulary_id,
            status: self.status,
            occurred_at_unix_ms: self.occurred_at_unix_ms,
        }
    }
}

/// Native facts for the live app's shared busy/error projection at integration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EffectiveStatusState {
    pub pending: bool,
    pub busy: bool,
    pub error: Option<DocumentError>,
}

#[derive(Default)]
pub(super) struct StatusState {
    attempt: u64,
    pending: Option<EffectiveStatusRequest>,
    in_flight: bool,
    error: Option<DocumentError>,
}

impl EffectiveReaderSession {
    pub fn status_state(&self) -> EffectiveStatusState {
        EffectiveStatusState {
            pending: self.status.pending.is_some(),
            busy: self.status.in_flight,
            error: self.status.error,
        }
    }

    /// The app additionally checks its shared FIFO/time/lifecycle gates. A new
    /// explicit choice may supersede a failed choice; retries never read edited
    /// values. Missing/deleted vocabulary has no enabled status buttons.
    pub fn begin_status_change(
        &mut self,
        selection: EffectiveReaderSelection,
        vocabulary_id: VocabularyId,
        status: VocabularyStatus,
        occurred_at_unix_ms: i64,
    ) -> Option<EffectiveStatusRequest> {
        if self.status.in_flight
            || self.words.blocks_completion()
            || self.completion_state().pending
            || occurred_at_unix_ms < 0
        {
            return None;
        }
        let command = self.freeze_selection(selection)?;
        let token = self
            .page()?
            .tokens
            .iter()
            .find(|token| token.segment_index == selection.segment_index)?;
        if token.vocabulary?.id != vocabulary_id {
            return None;
        }
        let attempt = self.status.attempt.checked_add(1)?;
        Some(self.start_status_request(EffectiveStatusRequest {
            attempt,
            command,
            vocabulary_id,
            surface: token.surface.clone(),
            status,
            occurred_at_unix_ms,
        }))
    }

    /// Retain exact spelling/provenance, ID, choice and timestamp across retries,
    /// even when the visible page has been invalidated. Storage revalidates them.
    pub fn retry_status_change(&mut self) -> Option<EffectiveStatusRequest> {
        if self.status.in_flight
            || self.completion_state().pending
            || self.words.blocks_completion()
        {
            return None;
        }
        let mut request = self.status.pending.clone()?;
        request.attempt = self.status.attempt.checked_add(1)?;
        Some(self.start_status_request(request))
    }

    /// An explicit Cancel can release a failed choice, never an in-flight write.
    pub fn cancel_status_retry(&mut self) -> bool {
        if self.status.in_flight || self.status.pending.is_none() {
            return false;
        }
        self.status.pending = None;
        self.status.error = None;
        true
    }

    pub fn finish_status_request(
        &mut self,
        request: &EffectiveStatusRequest,
        result: Result<Output, DocumentError>,
    ) -> bool {
        if !self.status.in_flight || self.status.pending.as_ref() != Some(request) {
            return false;
        }
        let selection = request.command.selection();
        let result = match result {
            Ok(Output::StatusChanged(returned, entry))
                if returned == selection
                    && entry.is_none_or(|entry| {
                        entry.id == request.vocabulary_id && entry.status == request.status
                    }) =>
            {
                Ok(entry)
            }
            Ok(_) => Err(DocumentError::InvalidDocument),
            Err(error) => Err(error),
        };
        self.status.in_flight = false;
        match result {
            Ok(entry) => {
                self.status.pending = None;
                self.status.error = None;
                // Apply live status/deletion to all repeated spellings and the
                // selected-word details only while this exact page is retained.
                if let Some(page) = self
                    .current
                    .as_mut()
                    .filter(|page| page.view.identity == selection.page)
                {
                    for token in &mut page.view.tokens {
                        if token.surface == request.surface {
                            token.vocabulary = entry;
                        }
                    }
                    self.observe_word_status(selection.page, &request.surface, entry);
                }
            }
            Err(error) => self.status.error = Some(error),
        }
        true
    }

    fn start_status_request(&mut self, request: EffectiveStatusRequest) -> EffectiveStatusRequest {
        self.status.attempt = request.attempt;
        self.status.pending = Some(request.clone());
        self.status.in_flight = true;
        self.status.error = None;
        request
    }
}
