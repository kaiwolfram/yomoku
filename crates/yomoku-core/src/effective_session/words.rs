//! Selected-word requests retain their native identity until the callback arrives.
use super::{
    EffectivePageToken, EffectiveReaderSelection, EffectiveReaderSession, EffectiveSelectionCommand,
};
use crate::{
    dictionary::DictionaryCandidates,
    effective_reader::EffectiveReaderWordDetails,
    effective_storage::{EffectiveStorageOperation as Operation, EffectiveStorageResult as Output},
    reader::ReaderVocabulary,
    storage::DocumentError,
    vocabulary::Surface,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveWordDetails {
    pub selection: EffectiveReaderSelection,
    pub token: EffectivePageToken,
    pub encountered: u64,
    pub candidates: DictionaryCandidates,
    pub dictionary_sources: Vec<crate::dictionary::DictionarySource>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct EffectiveReaderWordState {
    pub selection: Option<EffectiveReaderSelection>,
    pub details: Option<EffectiveWordDetails>,
    pub error: Option<DocumentError>,
    pub busy: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveWordRequest {
    attempt: u64,
    command: EffectiveSelectionCommand,
}
impl EffectiveWordRequest {
    pub fn operation(&self) -> Operation {
        Operation::LoadWord(self.command.clone())
    }
}
#[derive(Default)]
pub(super) struct WordState {
    attempt: u64,
    selection: Option<EffectiveReaderSelection>,
    details: Option<EffectiveWordDetails>,
    error: Option<DocumentError>,
    pending: Option<EffectiveWordRequest>,
}
impl WordState {
    pub(super) fn blocks_completion(&self) -> bool {
        self.pending.is_some()
    }
}
impl EffectiveReaderSession {
    pub fn mutation_pending(&self) -> bool {
        self.status_state().pending
    }
    pub(super) fn observe_word_status(
        &mut self,
        page: super::EffectivePageIdentity,
        surface: &str,
        vocabulary: Option<ReaderVocabulary>,
    ) {
        if let Some(details) = self
            .words
            .details
            .as_mut()
            .filter(|d| d.selection.page == page && d.token.surface == surface)
        {
            details.token.vocabulary = vocabulary;
            if vocabulary.is_none() {
                details.encountered = 0;
            }
        }
    }
    pub fn word_state(&self) -> EffectiveReaderWordState {
        // Keep the accepted storage result untouched. Each render projects the
        // current page again so taps/status changes cannot accumulate history.
        let details = self.words.details.as_ref().map(|saved| {
            let mut projected = saved.clone();
            let visible_occurrences = self
                .page()
                .filter(|page| page.identity == saved.selection.page)
                .map_or(0, |page| {
                    page.tokens
                        .iter()
                        .filter(|token| token.surface == saved.token.surface)
                        .count() as u64
                });
            projected.encountered = saved.encountered.saturating_add(visible_occurrences);
            projected
        });
        EffectiveReaderWordState {
            selection: self.words.selection,
            details,
            error: self.words.error,
            busy: self.words.pending.is_some(),
        }
    }
    pub fn dismiss_word(&mut self) {
        self.words.selection = None;
        self.words.details = None;
        self.words.error = None;
    }
    pub fn begin_word_load(
        &mut self,
        selection: EffectiveReaderSelection,
    ) -> Option<EffectiveWordRequest> {
        if self.words.pending.is_some()
            || self.completion_state().pending
            || self.status_state().pending
        {
            return None;
        }
        let command = self.freeze_selection(selection)?;
        self.words.attempt = self.words.attempt.checked_add(1)?;
        let request = EffectiveWordRequest {
            attempt: self.words.attempt,
            command,
        };
        self.dismiss_word();
        self.words.selection = Some(selection);
        self.words.pending = Some(request.clone());
        Some(request)
    }
    pub fn finish_word_request(
        &mut self,
        request: &EffectiveWordRequest,
        result: Result<Output, DocumentError>,
    ) -> bool {
        if self.words.pending.as_ref() != Some(request) {
            return false;
        }
        let selection = request.command.selection();
        let visible =
            self.words.selection == Some(selection) && self.resolve_selection(selection).is_some();
        let result = match result {
            Ok(Output::WordLoaded(returned, details)) if returned == selection => {
                self.compact_word(request, *details)
            }
            Ok(_) => Err(DocumentError::InvalidDocument),
            Err(error) => Err(error),
        };
        self.words.pending = None;
        if visible {
            match result {
                Ok(details) => {
                    for token in &mut self.current.as_mut().expect("visible page").view.tokens {
                        if token.surface == details.token.surface {
                            token.vocabulary = details.token.vocabulary;
                        }
                    }
                    self.words.details = Some(details);
                    self.words.error = None;
                }
                Err(error) => self.words.error = Some(error),
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

            dictionary_sources: details.dictionary_sources,
        })
    }
}
