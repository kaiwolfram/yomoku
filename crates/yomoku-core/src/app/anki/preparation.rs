//! Freeze only the current owner's selection. This data authorizes no I/O.

use super::*;
use crate::anki::{AnkiDeckId, AnkiExport, AnkiSelection};

impl Model {
    /// Native immutable intent, retained across popup/page changes. Before live
    /// use, the dispatcher still needs provider preflight and durable begin gates.
    pub fn prepared_anki_export(&self) -> Option<&AnkiExport> {
        self.prepared_anki_export.as_ref()
    }

    fn prepare_anki_export(
        &self,
        selection: AnkiSelection,
        deck: AnkiDeckId,
    ) -> Option<AnkiExport> {
        if self.prepared_anki_export.is_some()
            || !self.anki_export_gate_open()
            || self.storage_busy
            || self.settings_open
            || self.pending_time_checkpoint.is_some()
            || self.queued_time_checkpoint.is_some()
            || self.completion_pending()
            || self.deferred_reader_event.is_some()
            || self.effective_mutation_pending()
        {
            return None;
        }
        let document = self.reader_document.as_ref()?;
        match selection {
            AnkiSelection::Original(selection) => {
                if self.effective_reader.is_some()
                    || self.current_token_key() != Some(selection.key)
                    || self.reader_word_selection != Some(selection)
                    || self.reader_word_error.is_some()
                    || self.pending_word_load.is_some()
                    || self.pending_custom_change.is_some()
                    || document.document_id != selection.key.document_id
                {
                    return None;
                }
                let chapter = document.chapter(selection.key.page.chapter_id).ok()?;
                AnkiExport::from_original_selection(
                    deck,
                    chapter,
                    self.reader_tokens.as_ref()?,
                    selection,
                    self.reader_word.as_ref()?,
                )
                .ok()
            }
            AnkiSelection::Effective(selection) => {
                if !self.can_start_effective_selection(selection)
                    || document.document_id != selection.page.base.document_id
                {
                    return None;
                }
                let session = self.effective_reader.as_ref()?;
                let word = session.word_state();
                if word.selection != Some(selection) || word.busy || word.error.is_some() {
                    return None;
                }
                let chapter = document.chapter(selection.page.base.page.chapter_id).ok()?;
                AnkiExport::from_effective_selection(
                    deck,
                    chapter,
                    session.page()?,
                    selection,
                    word.details.as_ref()?,
                )
                .ok()
            }
        }
    }
}

impl Yomoku {
    pub(super) fn update_anki_preparation(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::PrepareAnkiExport { selection, deck } => {
                if let Some(export) = model.prepare_anki_export(*selection, *deck) {
                    model.prepared_anki_export = Some(export);
                }
            }
            // Only unsubmitted data exists here. Never extend this to clearing a
            // live request or recovery obligation when dispatch is connected.
            Event::ClearPreparedAnkiExport => {
                model.prepared_anki_export = None;
                // Preflight is read-only; invalidate its handle and observation.
                // Future submitted mutations must retain their separate journal.
                model.anki_export_preflight = None;
            }
            _ => return None,
        }
        Some(Command::done())
    }
}

#[cfg(test)]
mod tests;
