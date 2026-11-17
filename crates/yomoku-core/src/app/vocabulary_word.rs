//! Independent exact-surface lookups and retained direct status commands.

use super::*;
use crate::vocabulary::VocabularyWordState;

pub(super) struct VocabularyWordRequest {
    sequence: u64,
    operation: StorageOperation,
    obsolete: bool,
}

impl Model {
    fn dismiss_vocabulary_word(&mut self) {
        self.vocabulary.word = None;
        if let Some(request) = self.vocabulary_word_request_mut() {
            request.obsolete = true;
        }
    }
}

impl Yomoku {
    pub(super) fn update_vocabulary_word(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::DismissVocabularyWord => {
                model.dismiss_vocabulary_word();
                return Some(render::render());
            }
            Event::SelectDestination(_)
            | Event::OpenSettings
            | Event::OpenPaste
            | Event::OpenReader(_)
            | Event::CloseDocument => model.dismiss_vocabulary_word(),
            Event::CancelVocabularyWordStatus if !model.storage_busy() => {
                model.vocabulary.word_status = None;
                model.vocabulary_status_changes_membership = false;
                model.vocabulary.word_status_error = None;
                return Some(render::render());
            }
            Event::RetryVocabularyWordStatus => {
                if model.storage_busy() || model.vocabulary.word_status_error.is_none() {
                    return Some(Command::done());
                }
                let Some(command) = model.vocabulary.word_status.clone() else {
                    return Some(Command::done());
                };
                return Some(self.dispatch_vocabulary_word(
                    StorageOperation::SetVocabularyWordStatus(command),
                    model,
                ));
            }
            Event::OpenVocabularyWord(id)
                if model.vocabulary_visible()
                    && !model.storage_busy()
                    && model.vocabulary.word.is_none()
                    && model.vocabulary.word_status.is_none() =>
            {
                let Some(row) = model
                    .vocabulary
                    .rows
                    .iter()
                    .find(|row| row.id == *id)
                    .cloned()
                else {
                    return Some(Command::done());
                };
                let Some(generation) = model.vocabulary_request_sequence.checked_add(1) else {
                    return Some(Command::done());
                };
                model.vocabulary.word = Some(VocabularyWordState {
                    lookup: crate::dictionary::DictionaryLookup::default(),
                    selection: crate::anki::VocabularyAnkiSelection {
                        vocabulary_id: *id,
                        generation,
                    },
                    row,
                    candidates: None,
                    error: None,
                });
                return Some(
                    self.dispatch_vocabulary_word(StorageOperation::LoadVocabularyWord(*id), model),
                );
            }
            Event::RetryVocabularyWord
                if model.vocabulary_visible()
                    && !model.storage_busy()
                    && model.vocabulary.word_status.is_none() =>
            {
                let Some(word) = model.vocabulary.word.as_ref() else {
                    return Some(Command::done());
                };
                if word.error.is_none() {
                    return Some(Command::done());
                }
                return Some(self.dispatch_vocabulary_word(
                    StorageOperation::LoadVocabularyWord(word.row.id),
                    model,
                ));
            }
            Event::SetVocabularyWordStatus(command)
                if model.vocabulary_visible()
                    && !model.storage_busy()
                    && model.vocabulary.word_status.is_none() =>
            {
                let Some(word) = model.vocabulary.word.as_ref() else {
                    return Some(Command::done());
                };
                if word.row.id != command.id || command.occurred_at_unix_ms < 0 {
                    return Some(Command::done());
                }
                if word.row.status == command.status {
                    model.dismiss_vocabulary_word();
                    return Some(render::render());
                }
                model.vocabulary_status_changes_membership =
                    model.vocabulary.query.matches_status(word.row.status)
                        != model.vocabulary.query.matches_status(command.status);
                model.vocabulary.word_status = Some(command.clone());
                return Some(self.dispatch_vocabulary_word(
                    StorageOperation::SetVocabularyWordStatus(command.clone()),
                    model,
                ));
            }
            Event::OpenVocabularyWord(_)
            | Event::RetryVocabularyWord
            | Event::SetVocabularyWordStatus(_)
            | Event::CancelVocabularyWordStatus => return Some(Command::done()),
            // A failed direct write owns its frozen command until Retry/Cancel.
            Event::SetVocabularyQuery(_)
                if model.vocabulary.word_status.is_some() || model.vocabulary.word.is_some() =>
            {
                return Some(Command::done());
            }
            _ => {}
        }
        None
    }

    fn dispatch_vocabulary_word(
        &self,
        operation: StorageOperation,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        let Some(sequence) = model.vocabulary_request_sequence.checked_add(1) else {
            match operation {
                StorageOperation::SetVocabularyWordStatus(_) => {
                    model.vocabulary.word_status_error = Some(DocumentError::Unavailable)
                }
                _ => {
                    if let Some(word) = &mut model.vocabulary.word {
                        word.error = Some(DocumentError::Unavailable);
                    }
                }
            }
            return render::render();
        };
        model.vocabulary_request_sequence = sequence;
        match operation {
            StorageOperation::LoadVocabularyWord(_) => {
                if let Some(word) = &mut model.vocabulary.word {
                    word.error = None;
                    word.selection.generation = sequence;
                }
            }
            _ => model.vocabulary.word_status_error = None,
        }
        model.pending_storage = PendingStorage::VocabularyWord(VocabularyWordRequest {
            sequence,
            operation: operation.clone(),
            obsolete: false,
        });
        render::render().and(
            Command::request_from_shell(operation)
                .then_send(move |result| Event::VocabularyWordFinished(sequence, result)),
        )
    }

    pub(super) fn finish_vocabulary_word(
        &self,
        sequence: u64,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        if !model
            .vocabulary_word_request()
            .as_ref()
            .is_some_and(|request| request.sequence == sequence)
        {
            return Command::done();
        }
        let PendingStorage::VocabularyWord(request) = std::mem::take(&mut model.pending_storage)
        else {
            unreachable!("matched request")
        };
        match request.operation {
            StorageOperation::LoadVocabularyWord(id) => {
                if !request.obsolete && model.vocabulary_visible() {
                    if let Some(word) = model
                        .vocabulary
                        .word
                        .as_mut()
                        .filter(|word| word.row.id == id)
                    {
                        match result {
                            StorageResult::VocabularyWordLoaded(details)
                                if details.row.id == id
                                    && details.row.surface == word.row.surface
                                    && crate::dictionary::valid_parts(
                                        &word.row.surface,
                                        &details.parts,
                                    )
                                    && details.candidates.surface == word.row.surface =>
                            {
                                word.lookup = crate::dictionary::DictionaryLookup::new(
                                    &details.candidates,
                                    details.parts,
                                    word.selection.generation,
                                );
                                word.row = details.row;
                                word.candidates = Some(details.candidates);
                            }
                            StorageResult::Failed(error) => word.error = Some(error),
                            _ => word.error = Some(DocumentError::InvalidDocument),
                        }
                    }
                }
            }
            StorageOperation::SetVocabularyWordStatus(command) => {
                let error = match result {
                    StorageResult::VocabularyWordStatusChanged(ref changed)
                        if *changed == command =>
                    {
                        None
                    }
                    StorageResult::Failed(error) => Some(error),
                    _ => Some(DocumentError::InvalidDocument),
                };
                model.vocabulary.word_status_error = error;
                if error.is_none() {
                    model.vocabulary.word_status = None;
                    if !request.obsolete {
                        if let Some(word) = model
                            .vocabulary
                            .word
                            .as_mut()
                            .filter(|word| word.row.id == command.id)
                        {
                            word.row.status = command.status;
                        }
                    }
                    // The retained presentation may predate a failed prefix refresh. Use
                    // the accepted command's source status, even after dismissal/retry.
                    let changes_membership =
                        std::mem::take(&mut model.vocabulary_status_changes_membership);
                    if !changes_membership && model.vocabulary.query.matches_status(command.status)
                    {
                        model.patch_vocabulary_status(command.id, command.status);
                    }
                    model.invalidate_statistics();
                    model.invalidate_library_words();
                    model.invalidate_effective_page();
                    model.clear_reader_word();
                    if changes_membership && model.vocabulary.matching_count.is_some() {
                        if model.vocabulary_visible() {
                            return self.refresh_vocabulary_prefix(model);
                        }
                        model.invalidate_vocabulary();
                    }
                }
            }
            _ => unreachable!(),
        }
        render::render()
    }
}
