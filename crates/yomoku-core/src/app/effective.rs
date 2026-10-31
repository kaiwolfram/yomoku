//! Effective reader integration through the shared application gates.

use super::*;
use crate::effective_session::{EffectiveCompletionProgress, EffectiveReaderProgress};

impl Model {
    /// Native integration entry point, not a persisted setting or a shell toggle.
    /// Production uses this model. Legacy word/write events fail closed.
    pub fn with_effective_reader() -> Self {
        Self {
            effective_reader: Some(EffectiveReaderSession::default()),
            ..Self::default()
        }
    }

    pub(super) fn completion_pending(&self) -> bool {
        self.pending_completion.is_some()
            || self
                .effective_reader
                .as_ref()
                .is_some_and(|session| session.completion_state().pending)
    }

    pub(super) fn effective_mutation_pending(&self) -> bool {
        self.effective_reader
            .as_ref()
            .is_some_and(|session| session.mutation_pending())
    }

    pub(super) fn can_start_effective_selection(
        &self,
        selection: EffectiveReaderSelection,
    ) -> bool {
        self.can_start_effective_page(selection.page)
    }

    fn can_start_effective_page(&self, page: EffectivePageIdentity) -> bool {
        !self.storage_busy
            && !self.settings_open
            && self.pending_time_checkpoint.is_none()
            && self.queued_time_checkpoint.is_none()
            && !self.completion_pending()
            && self.deferred_reader_event.is_none()
            && self.current_token_key() == Some(page.base)
            && self.effective_reader.as_ref().is_some_and(|session| {
                session
                    .page()
                    .is_some_and(|current| current.identity == page)
            })
    }

    pub(super) fn invalidate_effective_page(&mut self) {
        if let Some(session) = &mut self.effective_reader {
            session.invalidate();
        }
    }

    pub(super) fn apply_completion(
        &mut self,
        completion: PageCompletion,
        document: ReaderDocument,
        outcome: AnalyzedCompletionOutcome,
    ) {
        self.invalidate_statistics();
        self.invalidate_vocabulary();
        self.invalidate_library_words();
        // The departing page determines Finish Book, including deliberate rereads.
        let final_page = document.chapters.last().is_some_and(|chapter| {
            completion.page.source_id == chapter.source_id
                && completion.page.chapter_id == chapter.chapter_id
                && completion.page.range.end.byte_index(&chapter.text) == Ok(chapter.text.len())
        });
        self.completion_notice = match outcome {
            AnalyzedCompletionOutcome::Recorded { newly_known } if newly_known > 0 => {
                Some(CompletionNotice {
                    command_id: completion.command_id,
                    newly_known,
                })
            }
            _ => None,
        };
        self.reader_time = None;
        self.reader_clock = None;
        self.apply_position(document);
        if final_page {
            self.close_document();
            self.destination = Destination::Library;
            self.settings_open = false;
            self.deferred_reader_event = None;
            if let AnalyzedCompletionOutcome::Recorded { newly_known } = outcome {
                self.book_completion_notice = Some(completion.command_id);
                self.completion_notice = (newly_known > 0).then_some(CompletionNotice {
                    command_id: completion.command_id,
                    newly_known,
                });
            }
        }
    }
}

impl Yomoku {
    pub(super) fn update_effective(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        if model.effective_reader.is_none() {
            return matches!(
                event,
                Event::CompleteEffectiveReaderPage { .. }
                    | Event::LoadEffectiveReaderWord(_)
                    | Event::SaveEffectiveReaderCustomEntry { .. }
                    | Event::DeleteEffectiveReaderCustomEntry(_)
                    | Event::RetryEffectiveReaderCustomEntry
                    | Event::CancelEffectiveReaderCustomEntry
                    | Event::SetEffectiveReaderStatus { .. }
                    | Event::RetryEffectiveReaderStatus
                    | Event::CancelEffectiveReaderStatus
                    | Event::EffectiveReaderOverride(_)
            )
            .then(Command::done);
        }
        // Until migrated, old token indices must never select/complete an
        // effective partition. This native opt-in cannot silently fall back to B.
        if matches!(
            event,
            Event::CompleteReaderPage { .. }
                | Event::SetReaderStatus(_)
                | Event::LoadReaderWord(_)
                | Event::SaveReaderCustomEntry { .. }
                | Event::DeleteReaderCustomEntry(_)
        ) {
            return Some(Command::done());
        }
        match event {
            Event::EffectiveReaderOverride(action) => {
                use EffectiveOverrideAction as Action;
                match action {
                    // Dismiss presentation even during I/O; accepted work keeps
                    // its native identity and the captured app FIFO obligation.
                    Action::Dismiss { .. } => {}
                    Action::Cancel { .. } => {
                        if model.storage_busy {
                            return Some(Command::done());
                        }
                    }
                    Action::Retry { .. } => {
                        if model.storage_busy
                            || model.pending_time_checkpoint.is_some()
                            || model.queued_time_checkpoint.is_some()
                        {
                            return Some(Command::done());
                        }
                    }
                    _ => {
                        let page = match action {
                            Action::Open { page, .. } => *page,
                            Action::Choose { identity, .. }
                            | Action::Save { identity, .. }
                            | Action::Generate { identity, .. }
                            | Action::Reverse { identity, .. } => identity.page,
                            _ => unreachable!(),
                        };
                        if !model.can_start_effective_page(page)
                            || model.effective_mutation_pending()
                        {
                            return Some(Command::done());
                        }
                    }
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .handle_override_action(action.clone());
                Some(if let Some(request) = request {
                    self.dispatch_effective(EffectiveReaderRequest::Override(request), model)
                } else {
                    self.drain_effective(model)
                })
            }
            Event::LoadEffectiveReaderWord(selection) => {
                if !model.can_start_effective_selection(*selection)
                    || model.effective_mutation_pending()
                {
                    return Some(Command::done());
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .begin_word_load(*selection);
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(EffectiveReaderRequest::Word(request), model)
                }))
            }
            Event::SaveEffectiveReaderCustomEntry {
                selection,
                english_meaning,
                reading,
            } => {
                if !model.can_start_effective_selection(*selection) {
                    return Some(Command::done());
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .begin_custom_entry_save(*selection, english_meaning.clone(), reading.clone());
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(EffectiveReaderRequest::Word(request), model)
                }))
            }
            Event::DeleteEffectiveReaderCustomEntry(selection) => {
                if !model.can_start_effective_selection(*selection) {
                    return Some(Command::done());
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .begin_custom_entry_delete(*selection);
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(EffectiveReaderRequest::Word(request), model)
                }))
            }
            Event::SetEffectiveReaderStatus {
                selection,
                vocabulary_id,
                status,
                occurred_at_unix_ms,
            } => {
                if !model.can_start_effective_selection(*selection) {
                    return Some(Command::done());
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .begin_status_change(*selection, *vocabulary_id, *status, *occurred_at_unix_ms);
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(EffectiveReaderRequest::Status(request), model)
                }))
            }
            Event::RetryEffectiveReaderCustomEntry | Event::RetryEffectiveReaderStatus => {
                if model.storage_busy
                    || model.pending_time_checkpoint.is_some()
                    || model.queued_time_checkpoint.is_some()
                {
                    return Some(Command::done());
                }
                let session = model.effective_reader.as_mut().expect("opt-in checked");
                let request = if matches!(event, Event::RetryEffectiveReaderCustomEntry) {
                    session
                        .retry_custom_entry()
                        .map(EffectiveReaderRequest::Word)
                } else {
                    session
                        .retry_status_change()
                        .map(EffectiveReaderRequest::Status)
                };
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(request, model)
                }))
            }
            Event::CancelEffectiveReaderCustomEntry | Event::CancelEffectiveReaderStatus => {
                if model.storage_busy {
                    return Some(Command::done());
                }
                let session = model.effective_reader.as_mut().expect("opt-in checked");
                let cancelled = if matches!(event, Event::CancelEffectiveReaderCustomEntry) {
                    session.cancel_custom_entry_retry()
                } else {
                    session.cancel_status_retry()
                };
                Some(if cancelled {
                    self.drain_effective(model)
                } else {
                    Command::done()
                })
            }
            Event::LoadReaderTokens(base) => {
                if model.storage_busy
                    || model.settings_open
                    || model.pending_time_checkpoint.is_some()
                    || model.queued_time_checkpoint.is_some()
                    || model.completion_pending()
                    || model.effective_mutation_pending()
                    || model.deferred_reader_event.is_some()
                    || model.current_token_key() != Some(*base)
                {
                    return Some(Command::done());
                }
                let session = model.effective_reader.as_mut().expect("opt-in checked");
                let Ok(request) = session.begin_load(*base) else {
                    model.document_error = Some(DocumentError::InvalidDocument);
                    return Some(render::render());
                };
                Some(self.dispatch_effective(EffectiveReaderRequest::Tokens(request), model))
            }
            Event::CompleteEffectiveReaderPage {
                identity,
                position,
                completed_at_unix_ms,
            } => {
                if model.completion_pending()
                    || model.deferred_reader_event.is_some()
                    || (model.storage_busy
                        && model.pending_time_checkpoint.is_none()
                        && model.queued_time_checkpoint.is_none())
                    || model.content_route != ContentRoute::Reader
                    || model.settings_open
                    || !model.reader.as_ref().is_some_and(|reader| {
                        reader.position.offset != reader.anchor.range.end
                            && reader.document_id == identity.base.document_id
                            && reader.anchor == identity.base.page
                            && reader.position == *position
                    })
                    || model.current_token_key() != Some(identity.base)
                {
                    return Some(Command::done());
                }
                let ReaderAnalysis::Available(analysis_id) = identity.base.analysis else {
                    model.document_error = Some(DocumentError::MissingAnalysis);
                    return Some(render::render());
                };
                let intent = PageCompletionIntent {
                    document_id: identity.base.document_id,
                    page: identity.base.page,
                    page_length: model.page_length.expect("opened reader settings"),
                    departing_position: *position,
                    completed_at_unix_ms: *completed_at_unix_ms,
                    analysis_id,
                };
                if !model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .begin_completion(*identity, intent)
                {
                    return Some(Command::done());
                }
                model.completion_notice = None;
                model.book_completion_notice = None;
                model.document_error = None;
                if model.pending_time_checkpoint.is_some() || model.queued_time_checkpoint.is_some()
                {
                    return Some(render::render());
                }
                Some(self.update(Event::RetryReaderCompletion, model))
            }
            Event::RetryReaderCompletion => {
                if model.storage_busy
                    || model.pending_time_checkpoint.is_some()
                    || model.queued_time_checkpoint.is_some()
                {
                    return Some(Command::done());
                }
                let request = model
                    .effective_reader
                    .as_mut()
                    .expect("opt-in checked")
                    .dispatch_completion();
                Some(request.map_or_else(Command::done, |request| {
                    self.dispatch_effective(EffectiveReaderRequest::Completion(request), model)
                }))
            }
            _ => None,
        }
    }

    fn dispatch_effective(
        &self,
        request: EffectiveReaderRequest,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        model.storage_busy = true;
        model.time_checkpoint_can_wait = matches!(
            request,
            EffectiveReaderRequest::Tokens(_)
                | EffectiveReaderRequest::Word(_)
                | EffectiveReaderRequest::Status(_)
                | EffectiveReaderRequest::Override(_)
        );
        model.effective_request = Some(request.clone());
        model.document_error = None;
        render::render().and(
            request.command(|request, result| {
                Event::EffectiveStorageFinished(Box::new(request), result)
            }),
        )
    }

    pub(super) fn finish_effective(
        &self,
        request: &EffectiveReaderRequest,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        if model.effective_request.as_ref() != Some(request) {
            return Command::done();
        }
        let progress = model
            .effective_reader
            .as_mut()
            .and_then(|session| session.finish_request(request, result));
        // A dismissed read has lost its presentation lifecycle but still owns the
        // app's FIFO slot. Release only its captured request, never newer work.
        model.effective_request = None;
        model.storage_busy = false;
        model.time_checkpoint_can_wait = false;
        match progress {
            Some(EffectiveReaderProgress::TokensFailed(error)) => {
                model.document_error = Some(error);
            }
            Some(
                EffectiveReaderProgress::WordFinished | EffectiveReaderProgress::StatusFinished,
            ) => {
                // Word reads/status writes can observe changes made by another session,
                // including deleted entries. Never retain a stale Library distribution.
                model.invalidate_library_words();
                model.invalidate_statistics();
                model.invalidate_vocabulary();
            }
            Some(EffectiveReaderProgress::Override(_)) => {
                // A successful write or stale/lost response may have changed
                // membership. A read can also observe another session's edits.
                model.invalidate_library_words();
                model.invalidate_statistics();
                model.invalidate_vocabulary();
            }
            Some(EffectiveReaderProgress::Completion(EffectiveCompletionProgress::Prepared)) => {
                return self.update(Event::RetryReaderCompletion, model);
            }
            Some(EffectiveReaderProgress::Completion(EffectiveCompletionProgress::Completed {
                completion,
                document,
                outcome,
            })) => model.apply_completion(completion, *document, outcome),
            Some(EffectiveReaderProgress::Completion(EffectiveCompletionProgress::Failed(
                error,
            ))) => {
                model.document_error = Some(error);
                return render::render();
            }
            _ => {}
        }
        self.drain_effective(model)
    }

    fn drain_effective(&self, model: &mut Model) -> Command<Effect, Event> {
        if model.pending_time_checkpoint.is_none() {
            if let Some(checkpoint) = model.queued_time_checkpoint {
                return self.update(Event::CheckpointReadingTime(checkpoint), model);
            }
            if !model.completion_pending() && !model.effective_mutation_pending() {
                if let Some(event) = model.deferred_reader_event.take() {
                    return self.update(event, model);
                }
            }
        }
        render::render()
    }
}
