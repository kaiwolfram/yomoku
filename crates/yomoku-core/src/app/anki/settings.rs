//! Independent local settings slot on the shared SQLite FIFO. Never owns reader
//! busy/errors, frozen exports, journal gates or provider invocation permits.

use super::*;
use crate::anki::{AnkiDeckId, AnkiSettingsPhase};

#[derive(Clone, Copy)]
pub(in crate::app) enum SettingsRequest {
    Load,
    Save(Option<AnkiDeckId>),
}

impl Yomoku {
    pub(in crate::app) fn update_anki_settings(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        use AnkiSettingsPhase::*;
        let request = match event {
            Event::LoadAnkiSettings if model.anki_settings.phase == Unloaded => {
                SettingsRequest::Load
            }
            Event::RetryAnkiSettings
                if model.settings_open
                    && matches!(model.anki_settings.phase, Failed | Loading | Saving) =>
            {
                // Read-only recovery queues behind the previous operation on the
                // same FIFO. Even a committed write with a lost response cannot
                // be replayed by Retry; old callbacks lose their request ID.
                SettingsRequest::Load
            }
            Event::SetAnkiDefaultDeck(value)
                if model.settings_open && model.anki_settings.phase == Ready =>
            {
                let Ok(deck) = value.map(AnkiDeckId::try_from).transpose() else {
                    return Some(Command::done());
                };
                if *value == model.anki_settings.default_deck_id {
                    return Some(Command::done());
                }
                SettingsRequest::Save(deck)
            }
            Event::LoadAnkiSettings | Event::RetryAnkiSettings | Event::SetAnkiDefaultDeck(_) => {
                return Some(Command::done());
            }
            Event::AnkiSettingsFinished(id, result) => {
                let Some((pending_id, request)) = model.anki_settings_request else {
                    return Some(Command::done());
                };
                if *id != pending_id {
                    return Some(Command::done());
                }
                model.anki_settings_request = None;
                let result = match (request, result) {
                    (SettingsRequest::Load, StorageResult::AnkiSettingsLoaded(deck)) => Ok(*deck),
                    (
                        SettingsRequest::Save(expected),
                        StorageResult::AnkiDefaultDeckSaved(deck),
                    ) if *deck == expected => Ok(*deck),
                    (_, StorageResult::Failed(error)) => Err(*error),
                    _ => Err(DocumentError::Unavailable),
                };
                match result {
                    Ok(deck) => {
                        if model.anki_settings.default_deck_id != deck.map(i64::from) {
                            model.anki_setup.phase = crate::anki::AnkiSetupPhase::Unchecked;
                            model.anki_entries.clear();
                        }
                        model.anki_settings.default_deck_id = deck.map(i64::from);
                        model.anki_settings.phase = Ready;
                        model.anki_settings.storage_error = None;
                    }
                    Err(error) => {
                        model.anki_settings.phase = Failed;
                        model.anki_settings.storage_error = Some(error);
                    }
                }
                return Some(render::render());
            }
            _ => return None,
        };
        let Some(id) = model.anki_settings_sequence.checked_add(1) else {
            model.anki_settings_request = None;
            model.anki_settings.phase = Failed;
            model.anki_settings.storage_error = Some(DocumentError::Unavailable);
            return Some(render::render());
        };
        model.anki_settings_sequence = id;
        model.anki_settings_request = Some((id, request));
        model.anki_settings.storage_error = None;
        let operation = match request {
            SettingsRequest::Load => {
                model.anki_settings.phase = Loading;
                StorageOperation::LoadAnkiSettings
            }
            SettingsRequest::Save(deck) => {
                model.anki_settings.phase = Saving;
                StorageOperation::SaveAnkiDefaultDeck(deck)
            }
        };
        Some(
            render::render().and(
                Command::request_from_shell(operation)
                    .then_send(move |result| Event::AnkiSettingsFinished(id, result)),
            ),
        )
    }
}

#[cfg(test)]
mod tests;
