//! In-memory Anki preparation, observations and active-operation ownership.
use super::*;
pub(super) mod entries;
mod export_view;
mod live;
mod preflight;
mod preparation;
pub(super) mod settings;
#[cfg(test)]
mod test_support;
impl Yomoku {
    pub(super) fn update_anki(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        self.update_anki_entries(event, model)
            .or_else(|| self.update_anki_live(event, model))
            .or_else(|| self.update_anki_preflight(event, model))
            .or_else(|| self.update_anki_preparation(event, model))
    }
}
