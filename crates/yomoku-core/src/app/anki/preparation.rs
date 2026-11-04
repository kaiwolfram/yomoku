//! Freeze only the current owner's selection. This data authorizes no I/O.

use super::*;
use crate::anki::{AnkiDeckId, AnkiExport, AnkiSelection, AnkiSettingsPhase};

impl Model {
    /// Native immutable intent, retained across popup/page changes. Before live
    /// use, the dispatcher still needs provider preflight and an active owner.
    pub fn prepared_anki_export(&self) -> Option<&AnkiExport> {
        self.prepared_anki_export.as_ref()
    }

    pub(super) fn prepare_anki_export(
        &self,
        selection: AnkiSelection,
        deck: AnkiDeckId,
        entry: Option<crate::anki::AnkiEntryKey>,
    ) -> Option<AnkiExport> {
        if self.storage_busy
            || self.settings_open
            || self.pending_time_checkpoint.is_some()
            || self.queued_time_checkpoint.is_some()
            || self.completion_pending()
            || self.deferred_reader_event.is_some()
            || self.effective_mutation_pending()
        {
            return None;
        }
        let AnkiSelection::Effective(current) = selection;
        if !self.can_start_effective_selection(current) {
            return None;
        }
        self.project_anki_export(selection, deck, entry)
    }

    /// Pure observation payload: a reader time save cannot suppress fresh provider queries.
    /// Explicit write preparation still passes all the mutation gates above.
    pub(super) fn project_anki_export(
        &self,
        selection: AnkiSelection,
        deck: AnkiDeckId,
        entry: Option<crate::anki::AnkiEntryKey>,
    ) -> Option<AnkiExport> {
        if self.settings_open || self.content_route != ContentRoute::Reader {
            return None;
        }
        let document = self.reader_document.as_ref()?;
        match selection {
            AnkiSelection::Effective(selection) => {
                if self.current_token_key() != Some(selection.page.base)
                    || self.effective_reader.page()?.identity != selection.page
                    || document.document_id != selection.page.base.document_id
                {
                    return None;
                }
                let session = &self.effective_reader;
                let word = session.word_state();
                if word.selection != Some(selection) || word.busy || word.error.is_some() {
                    return None;
                }
                let chapter = document.chapter(selection.page.base.page.chapter_id).ok()?;
                if let Some(entry) = entry {
                    let note = crate::anki::AnkiNote::from_effective_entry(
                        chapter,
                        session.page()?,
                        selection,
                        word.details.as_ref()?,
                        Some(entry),
                    )
                    .ok()?;
                    return Some(AnkiExport::for_entry(
                        AnkiSelection::Effective(selection),
                        entry,
                        deck,
                        note,
                    ));
                }
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
            Event::PrepareReaderAnkiExport { selection } => {
                if model.anki_settings.phase != AnkiSettingsPhase::Ready {
                    return Some(Command::done());
                }
                let Some(deck) = model.anki_settings.default_deck_id else {
                    return Some(Command::done());
                };
                let Ok(deck) = AnkiDeckId::try_from(deck) else {
                    return Some(Command::done());
                };
                return self.update_anki_preparation(
                    &Event::PrepareAnkiExport {
                        selection: *selection,
                        deck,
                    },
                    model,
                );
            }
            Event::ClearAnkiExport { preparation_id } => {
                if model.anki_export_view().is_none()
                    || model.anki_preparation_sequence != *preparation_id
                {
                    return Some(Command::done());
                }
                return self.update_anki_preparation(&Event::ClearPreparedAnkiExport, model);
            }
            Event::PrepareAnkiExport { selection, deck } => {
                let Some(id) = model.anki_preparation_sequence.checked_add(1) else {
                    return Some(Command::done());
                };
                if model.prepared_anki_export.is_some() || !model.anki_export_gate_open() {
                    return Some(Command::done());
                }
                if let Some(export) = model.prepare_anki_export(*selection, *deck, None) {
                    model.anki_preparation_sequence = id;
                    model.prepared_anki_export = Some(export);
                }
            }
            // Clear unsubmitted intent or dismiss a finished live presentation.
            // Active provider work must finish before intent can be cleared.
            Event::ClearPreparedAnkiExport => {
                if model.anki_operation_busy() {
                    return Some(Command::done());
                }
                model.anki_live = None;
                model.prepared_anki_export = None;
                // Preflight is read-only; invalidate its handle and observation.
                // A later explicit action must capture a fresh observation.
                model.anki_export_preflight = None;
            }
            _ => return None,
        }
        Some(Command::done())
    }
}
