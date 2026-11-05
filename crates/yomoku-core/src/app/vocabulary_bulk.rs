//! Frozen vocabulary selections use the same single storage slot as the browser.

use super::*;
use crate::vocabulary::{VocabularyBulkAction, VocabularyBulkCommand, VocabularyQuery};

#[derive(Clone)]
pub(super) enum VocabularyBulkRequest {
    SelectAll(VocabularyQuery),
    Apply(VocabularyBulkCommand),
}

impl Yomoku {
    pub(super) fn update_vocabulary_bulk(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        let bulk_event = matches!(
            event,
            Event::ToggleVocabularySelection(_)
                | Event::SelectAllVocabulary
                | Event::RetryVocabularySelection
                | Event::CancelVocabularySelection
                | Event::RequestVocabularyBulkAction(_)
                | Event::ConfirmVocabularyBulkAction(_)
                | Event::RetryVocabularyBulkAction
                | Event::CancelVocabularyBulkAction
        );
        // A prompt or failed command owns its frozen payload until Cancel or
        // explicit Retry. In-flight work cannot be dismissed or superseded.
        let frozen = model.vocabulary_bulk_request.is_some()
            || model.vocabulary_select_all_query.is_some()
            || model.vocabulary.confirmation.is_some();
        if !bulk_event {
            if frozen {
                return Some(Command::done());
            }
            if matches!(
                event,
                Event::SelectDestination(_) | Event::OpenPaste | Event::CloseDocument
            ) {
                model.vocabulary.selection = None;
                model.vocabulary.bulk_error = None;
            }
            return None;
        }
        if !model.vocabulary_visible() || model.storage_busy {
            return Some(Command::done());
        }
        let state = &mut model.vocabulary;
        match event {
            Event::CancelVocabularyBulkAction if state.confirmation.is_some() => {
                state.confirmation = None;
                state.bulk_error = None;
            }
            Event::CancelVocabularySelection if state.confirmation.is_none() => {
                state.selection = None;
                state.bulk_error = None;
                model.vocabulary_select_all_query = None;
            }
            Event::RetryVocabularySelection
                if model.vocabulary_select_all_query.is_some() && state.bulk_error.is_some() =>
            {
                let query = model
                    .vocabulary_select_all_query
                    .clone()
                    .expect("frozen query");
                return Some(
                    self.dispatch_vocabulary_bulk(VocabularyBulkRequest::SelectAll(query), model),
                );
            }
            Event::ConfirmVocabularyBulkAction(command)
                if state.confirmation.as_ref() == Some(command) && state.bulk_error.is_none() =>
            {
                return Some(self.dispatch_vocabulary_bulk(
                    VocabularyBulkRequest::Apply(command.clone()),
                    model,
                ));
            }
            Event::RetryVocabularyBulkAction
                if state.confirmation.is_some() && state.bulk_error.is_some() =>
            {
                let command = state.confirmation.clone().expect("frozen command");
                return Some(
                    self.dispatch_vocabulary_bulk(VocabularyBulkRequest::Apply(command), model),
                );
            }
            _ if frozen => return Some(Command::done()),
            Event::ToggleVocabularySelection(id) if state.rows.iter().any(|row| row.id == *id) => {
                let ids = state.selection.get_or_insert_with(Vec::new);
                match ids.binary_search(id) {
                    Ok(index) => {
                        ids.remove(index);
                    }
                    Err(index) => ids.insert(index, *id),
                }
            }
            Event::SelectAllVocabulary
                if state.matching_count.is_some() && state.error.is_none() =>
            {
                let query = state.query.clone();
                model.vocabulary_select_all_query = Some(query.clone());
                return Some(
                    self.dispatch_vocabulary_bulk(VocabularyBulkRequest::SelectAll(query), model),
                );
            }
            Event::RequestVocabularyBulkAction(action) => {
                if matches!(action, VocabularyBulkAction::SetStatus { occurred_at_unix_ms, .. } if *occurred_at_unix_ms < 0)
                {
                    return Some(Command::done());
                }
                let Some(ids) = state.selection.as_ref().filter(|ids| !ids.is_empty()) else {
                    return Some(Command::done());
                };
                state.confirmation = Some(VocabularyBulkCommand {
                    ids: ids.clone(),
                    action: *action,
                });
                state.bulk_error = None;
            }
            _ => return Some(Command::done()),
        }
        Some(render::render())
    }

    fn dispatch_vocabulary_bulk(
        &self,
        request: VocabularyBulkRequest,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        let Some(id) = model.vocabulary_request_sequence.checked_add(1) else {
            model.vocabulary.bulk_error = Some(DocumentError::Unavailable);
            return render::render();
        };
        model.vocabulary_request_sequence = id;
        let operation = match &request {
            VocabularyBulkRequest::SelectAll(query) => {
                StorageOperation::VocabularyMatchingIds(query.clone())
            }
            VocabularyBulkRequest::Apply(command) => {
                StorageOperation::ApplyVocabularyBulk(command.clone())
            }
        };
        model.vocabulary.selecting_all = matches!(request, VocabularyBulkRequest::SelectAll(_));
        model.vocabulary.bulk_error = None;
        model.vocabulary_bulk_request = Some((id, request));
        model.storage_busy = true;

        render::render().and(
            Command::request_from_shell(operation)
                .then_send(move |result| Event::VocabularyBulkFinished(id, result)),
        )
    }

    pub(super) fn finish_vocabulary_bulk(
        &self,
        id: u64,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        if !model
            .vocabulary_bulk_request
            .as_ref()
            .is_some_and(|(pending, _)| *pending == id)
        {
            return Command::done();
        }
        let (_, request) = model
            .vocabulary_bulk_request
            .take()
            .expect("matched request");
        model.storage_busy = false;
        model.vocabulary.selecting_all = false;
        match (request, result) {
            (VocabularyBulkRequest::SelectAll(_), StorageResult::VocabularyMatchingIds(ids))
                if ids.windows(2).all(|pair| pair[0] < pair[1]) =>
            {
                let selected = model.vocabulary.selection.get_or_insert_with(Vec::new);
                selected.extend(ids);
                selected.sort_unstable();
                selected.dedup();
                model.vocabulary_select_all_query = None;
            }
            (
                VocabularyBulkRequest::Apply(command),
                StorageResult::VocabularyBulkApplied(applied),
            ) if command == applied => {
                model.vocabulary.selection = None;
                model.vocabulary.confirmation = None;
                model.vocabulary.bulk_error = None;
                model.invalidate_vocabulary();
                model.invalidate_statistics();
                model.invalidate_library_words();
                model.invalidate_effective_page();
                model.clear_reader_word();
            }
            (_, StorageResult::Failed(error)) => model.vocabulary.bulk_error = Some(error),
            _ => model.vocabulary.bulk_error = Some(DocumentError::InvalidDocument),
        }
        render::render()
    }
}
