//! Selection state and accepted commands stay native until a result is compacted.

use serde::{Deserialize, Serialize};

use super::{
    EffectivePageToken, EffectiveReaderSelection, EffectiveReaderSession, EffectiveSelectionCommand,
};
use crate::{
    dictionary::{CustomDictionaryEntry, DictionaryCandidates},
    effective_reader::EffectiveReaderWordDetails,
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    reader::{ReaderCustomEntry, ReaderVocabulary},
    storage::DocumentError,
    vocabulary::Surface,
};

/// Shell-facing details contain one visible token, never the resolved chapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveWordDetails {
    pub selection: EffectiveReaderSelection,
    pub token: EffectivePageToken,
    pub encountered: u64,
    pub candidates: DictionaryCandidates,
    pub custom_entry: Option<ReaderCustomEntry>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveReaderWordState {
    pub selection: Option<EffectiveReaderSelection>,
    pub details: Option<EffectiveWordDetails>,
    pub error: Option<DocumentError>,
    pub custom_entry_error: Option<DocumentError>,
    pub busy: bool,
    /// Includes a failed accepted save/delete awaiting an explicit exact retry.
    pub custom_entry_pending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Load,
    Save {
        english_meaning: String,
        reading: String,
    },
    Delete,
}

/// Rust-only callback token. Each dispatch, including an exact-payload retry,
/// gets a fresh attempt so a duplicate old result cannot consume the retry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveWordRequest {
    attempt: u64,
    command: EffectiveSelectionCommand,
    action: Action,
}

impl EffectiveWordRequest {
    pub fn operation(&self) -> Operation {
        match &self.action {
            Action::Load => Operation::LoadWord(self.command.clone()),
            Action::Save {
                english_meaning,
                reading,
            } => Operation::SaveCustomEntry {
                command: self.command.clone(),
                english_meaning: english_meaning.clone(),
                reading: reading.clone(),
            },
            Action::Delete => Operation::DeleteCustomEntry(self.command.clone()),
        }
    }
}

#[derive(Default)]
pub(super) struct WordState {
    attempt: u64,
    selection: Option<EffectiveReaderSelection>,
    details: Option<EffectiveWordDetails>,
    error: Option<DocumentError>,
    custom_error: Option<DocumentError>,
    pending: Option<EffectiveWordRequest>,
    in_flight: bool,
}

impl WordState {
    pub(super) fn blocks_completion(&self) -> bool {
        self.in_flight
            || self
                .pending
                .as_ref()
                .is_some_and(|request| request.action != Action::Load)
    }
}

impl EffectiveReaderSession {
    pub(super) fn observe_word_status(
        &mut self,
        page: super::EffectivePageIdentity,
        surface: &str,
        vocabulary: Option<ReaderVocabulary>,
    ) {
        if let Some(details) =
            self.words.details.as_mut().filter(|details| {
                details.selection.page == page && details.token.surface == surface
            })
        {
            details.token.vocabulary = vocabulary;
        }
    }

    pub fn word_state(&self) -> EffectiveReaderWordState {
        EffectiveReaderWordState {
            selection: self.words.selection,
            details: self.words.details.clone(),
            error: self.words.error,
            custom_entry_error: self.words.custom_error,
            busy: self.words.in_flight,
            custom_entry_pending: self
                .words
                .pending
                .as_ref()
                .is_some_and(|request| request.action != Action::Load),
        }
    }

    /// Dismissal retires presentation, while accepted I/O still needs its callback
    /// or explicit retry. Page invalidation calls this before dropping its key.
    pub fn dismiss_word(&mut self) {
        self.words.selection = None;
        self.words.details = None;
        self.words.error = None;
        self.words.custom_error = None;
    }

    /// The app also applies its shared FIFO/time/completion busy gates before
    /// calling this. Lookup failures do not automatically dispatch another read.
    pub fn begin_word_load(
        &mut self,
        selection: EffectiveReaderSelection,
    ) -> Option<EffectiveWordRequest> {
        if self.words.in_flight
            || self.completion_state().pending
            || self.status_state().pending
            || self.overrides.blocks_reader()
        {
            return None;
        }
        let command = self.freeze_selection(selection)?;
        let request = self.word_request(command, Action::Load)?;
        self.dismiss_word();
        self.words.selection = Some(selection);
        self.start_word_request(request.clone());
        Some(request)
    }

    pub fn begin_custom_entry_save(
        &mut self,
        selection: EffectiveReaderSelection,
        english_meaning: String,
        reading: String,
    ) -> Option<EffectiveWordRequest> {
        let command = self.custom_entry_command(selection)?;
        // Preserve literal Unicode/whitespace; do not expose invalid drafts to I/O.
        CustomDictionaryEntry::new(
            Surface::new(&self.words.details.as_ref()?.token.surface).ok()?,
            english_meaning.clone(),
            reading.clone(),
        )
        .ok()?;
        let request = self.word_request(
            command,
            Action::Save {
                english_meaning,
                reading,
            },
        )?;
        self.start_word_request(request.clone());
        Some(request)
    }

    pub fn begin_custom_entry_delete(
        &mut self,
        selection: EffectiveReaderSelection,
    ) -> Option<EffectiveWordRequest> {
        let command = self.custom_entry_command(selection)?;
        self.words.details.as_ref()?.custom_entry.as_ref()?;
        let request = self.word_request(command, Action::Delete)?;
        self.start_word_request(request.clone());
        Some(request)
    }

    /// Retry the accepted key and literal values, even after UI invalidation.
    /// Storage revalidates provenance; it never substitutes the new visible word.
    pub fn retry_custom_entry(&mut self) -> Option<EffectiveWordRequest> {
        if self.words.in_flight
            || self.completion_state().pending
            || self.status_state().pending
            || self.overrides.blocks_reader()
        {
            return None;
        }
        let pending = self.words.pending.as_ref()?;
        if pending.action == Action::Load {
            return None;
        }
        let request = self.word_request(pending.command.clone(), pending.action.clone())?;
        self.start_word_request(request.clone());
        Some(request)
    }

    /// Explicit Cancel may abandon a failed retry without changing the last
    /// committed entry. An in-flight write still has to finish before Cancel.
    pub fn cancel_custom_entry_retry(&mut self) -> bool {
        if self.words.in_flight
            || !self
                .words
                .pending
                .as_ref()
                .is_some_and(|request| request.action != Action::Load)
        {
            return false;
        }
        self.words.pending = None;
        self.words.custom_error = None;
        true
    }

    /// Consume the request captured by the native callback, not one reconstructed
    /// from a shell selection. A stale callback cannot clear a newer obligation.
    pub fn finish_word_request(
        &mut self,
        request: &EffectiveWordRequest,
        result: Result<Output, DocumentError>,
    ) -> bool {
        if !self.words.in_flight || self.words.pending.as_ref() != Some(request) {
            return false;
        }
        let selection = request.command.selection();
        let visible =
            self.words.selection == Some(selection) && self.resolve_selection(selection).is_some();
        let result = match result {
            Ok(Output::WordLoaded(returned, details))
                if request.action == Action::Load && returned == selection =>
            {
                self.compact_word(request, *details).map(Some)
            }
            Ok(Output::CustomEntryChanged(returned, entry))
                if returned == selection
                    && Self::matches_custom_result(request, entry.as_ref()) =>
            {
                Ok(None)
            }
            Ok(_) => Err(DocumentError::InvalidDocument),
            Err(error) => Err(error),
        };
        self.words.in_flight = false;
        match result {
            Ok(details) => {
                self.words.pending = None;
                if visible {
                    if let Some(details) = details {
                        // A read observes live status, including deletion. Repeated
                        // spellings on this page must show the same observation.
                        for token in &mut self.current.as_mut().expect("visible page").view.tokens {
                            if token.surface == details.token.surface {
                                token.vocabulary = details.token.vocabulary;
                            }
                        }
                        self.words.details = Some(details);
                        self.words.error = None;
                    } else if let Some(details) = self.words.details.as_mut() {
                        details.custom_entry = match &request.action {
                            Action::Save {
                                english_meaning,
                                reading,
                            } => Some(ReaderCustomEntry {
                                surface: details.token.surface.clone(),
                                english_meaning: english_meaning.clone(),
                                reading: reading.clone(),
                            }),
                            Action::Delete => None,
                            Action::Load => unreachable!(),
                        };
                    }
                    self.words.custom_error = None;
                }
            }
            Err(error) if request.action == Action::Load => {
                self.words.pending = None;
                if visible {
                    self.words.error = Some(error);
                }
            }
            Err(error) => {
                // Keep the frozen command for explicit retry; the last committed
                // entry remains displayed until success.
                if visible {
                    self.words.custom_error = Some(error);
                }
            }
        }
        true
    }

    fn compact_word(
        &self,
        request: &EffectiveWordRequest,
        details: EffectiveReaderWordDetails,
    ) -> Result<EffectiveWordDetails, DocumentError> {
        let selection = request.command.selection();
        if &details.key != request.command.key()
            || details.token.segment_index != selection.segment_index
            || details.candidates.surface != details.token.surface
            || details.candidates.dictionary_form.as_deref()
                != details
                    .token
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.dictionary_form.as_ref().map(Surface::as_str))
            || details.custom_entry.as_ref().is_some_and(|entry| {
                entry.surface != details.token.surface
                    || entry.english_meaning.trim().is_empty()
                    || entry.reading.trim().is_empty()
                    || !details.candidates.words.is_empty()
                    || !details.candidates.names.is_empty()
            })
        {
            return Err(DocumentError::InvalidDocument);
        }
        let token = EffectivePageToken::from(details.token);
        // Authenticate all visible fields against the retained page; vocabulary
        // alone is allowed to have changed since the page read.
        if self.words.selection == Some(selection) {
            let expected = self
                .page()
                .filter(|page| page.identity == selection.page)
                .and_then(|page| {
                    page.tokens
                        .iter()
                        .find(|value| value.segment_index == selection.segment_index)
                });
            if !expected.is_some_and(|expected| {
                let mut expected = expected.clone();
                expected.vocabulary = token.vocabulary;
                expected == token
            }) {
                return Err(DocumentError::InvalidDocument);
            }
        }
        Ok(EffectiveWordDetails {
            selection,
            token,
            encountered: details.encountered,
            candidates: details.candidates,
            custom_entry: details.custom_entry,
        })
    }

    fn matches_custom_result(
        request: &EffectiveWordRequest,
        entry: Option<&ReaderCustomEntry>,
    ) -> bool {
        match (&request.action, entry) {
            (
                Action::Save {
                    english_meaning,
                    reading,
                },
                Some(entry),
            ) => {
                let selection = request.command.selection();
                let surface = request
                    .command
                    .key()
                    .segmentation
                    .segments
                    .get(match usize::try_from(selection.segment_index) {
                        Ok(index) => index,
                        Err(_) => return false,
                    })
                    .map(|segment| segment.boundary.surface.as_str());
                surface == Some(entry.surface.as_str())
                    && &entry.english_meaning == english_meaning
                    && &entry.reading == reading
            }
            (Action::Delete, None) => true,
            _ => false,
        }
    }

    fn custom_entry_command(
        &self,
        selection: EffectiveReaderSelection,
    ) -> Option<EffectiveSelectionCommand> {
        if self.words.in_flight
            || self.completion_state().pending
            || self.status_state().pending
            || self.overrides.blocks_reader()
            || self.words.selection != Some(selection)
        {
            return None;
        }
        let details = self.words.details.as_ref()?;
        if details.selection != selection
            || !details.candidates.words.is_empty()
            || !details.candidates.names.is_empty()
        {
            return None;
        }
        self.freeze_selection(selection)
    }

    fn word_request(
        &mut self,
        command: EffectiveSelectionCommand,
        action: Action,
    ) -> Option<EffectiveWordRequest> {
        self.words.attempt = self.words.attempt.checked_add(1)?;
        Some(EffectiveWordRequest {
            attempt: self.words.attempt,
            command,
            action,
        })
    }

    fn start_word_request(&mut self, request: EffectiveWordRequest) {
        self.words.pending = Some(request);
        self.words.in_flight = true;
        self.words.custom_error = None;
    }
}
