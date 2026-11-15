//! Shared Reader/Vocabulary choice validation and read-only lookup ownership.
use super::*;
use crate::{
    anki::AnkiSelection,
    dictionary::{DictionaryCandidates, DictionaryLookup},
};

impl Model {
    fn dictionary_word_mut(
        &mut self,
        selection: AnkiSelection,
    ) -> Option<(&mut DictionaryLookup, &mut DictionaryCandidates)> {
        match selection {
            AnkiSelection::Effective {
                selection,
                lookup_revision,
            } => {
                if self.settings_open || self.content_route != ContentRoute::Reader {
                    return None;
                }
                let details = self.effective_reader.dictionary_word_mut(selection)?;
                if details.lookup.revision != lookup_revision {
                    return None;
                }
                Some((&mut details.lookup, &mut details.candidates))
            }
            AnkiSelection::Vocabulary(selection) => {
                if !self.vocabulary_visible() {
                    return None;
                }
                let word = self
                    .vocabulary
                    .word
                    .as_mut()
                    .filter(|word| word.selection == selection)?;
                Some((&mut word.lookup, word.candidates.as_mut()?))
            }
        }
    }
}

impl Yomoku {
    pub(super) fn update_dictionary_lookup(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        let (selection, part, retry) = match event {
            Event::ChooseDictionaryPart { selection, part } => (*selection, *part, false),
            Event::RetryDictionaryPart(selection) => (*selection, None, true),
            _ => return None,
        };
        if model.storage_busy()
            || model.completion_pending()
            || model.effective_mutation_pending()
            || model.vocabulary.word_status.is_some()
            || model.deferred_reader_event.is_some()
        {
            return Some(Command::done());
        }
        let Some((lookup, _)) = model.dictionary_word_mut(selection) else {
            return Some(Command::done());
        };
        let part = if retry { lookup.active } else { part };
        if lookup.loading || (retry && lookup.error.is_none()) || (!retry && part == lookup.active)
        {
            return Some(Command::done());
        }
        let Some(query) = lookup.query(part) else {
            return Some(Command::done());
        };
        let revision = match selection {
            AnkiSelection::Effective { .. } => model.effective_reader.next_dictionary_revision(),
            AnkiSelection::Vocabulary(_) => model
                .vocabulary_request_sequence
                .checked_add(1)
                .inspect(|sequence| model.vocabulary_request_sequence = *sequence),
        };
        let Some(revision) = revision else {
            return Some(Command::done());
        };
        let Some((lookup, _)) = model.dictionary_word_mut(selection) else {
            return Some(Command::done());
        };
        lookup.active = part;
        lookup.revision = revision;
        lookup.loading = true;
        lookup.error = None;
        let selection = match selection {
            AnkiSelection::Effective { selection, .. } => AnkiSelection::Effective {
                selection,
                lookup_revision: revision,
            },
            AnkiSelection::Vocabulary(mut selection) => {
                selection.generation = revision;
                model
                    .vocabulary
                    .word
                    .as_mut()
                    .expect("validated word")
                    .selection = selection;
                AnkiSelection::Vocabulary(selection)
            }
        };
        model.pending_storage = PendingStorage::DictionaryLookup(selection, query.clone());
        Some(
            render::render().and(
                Command::request_from_shell(StorageOperation::LookupDictionary(query))
                    .then_send(move |result| Event::DictionaryLookupFinished(selection, result)),
            ),
        )
    }

    pub(super) fn finish_dictionary_lookup(
        &self,
        selection: AnkiSelection,
        result: StorageResult,
        model: &mut Model,
    ) -> Command<Effect, Event> {
        let PendingStorage::DictionaryLookup(owner, query) = &model.pending_storage else {
            return Command::done();
        };
        if *owner != selection {
            return Command::done();
        }
        let query = query.clone();
        model.pending_storage = PendingStorage::Idle;
        if let Some((lookup, candidates)) = model.dictionary_word_mut(selection) {
            lookup.loading = false;
            match result {
                StorageResult::DictionaryLoaded(found)
                    if found.surface == query.surface
                        && found.dictionary_form == query.dictionary_form =>
                {
                    *candidates = found;
                    lookup.error = None;
                }
                StorageResult::Failed(error) => lookup.error = Some(error),
                _ => lookup.error = Some(DocumentError::InvalidDocument),
            }
        }
        render::render()
    }
}
