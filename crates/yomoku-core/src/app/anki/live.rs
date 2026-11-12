//! One in-memory owner per provider write. No SQLite begin, finish, or replay.
use super::*;
use crate::anki::{
    AnkiLiveIntent, AnkiLiveInvocation, AnkiLiveProgress, AnkiLiveRequest, AnkiLiveState,
    AnkiModelCreation, AnkiPreflightPlan, AnkiSubmission,
};
impl Model {
    pub fn anki_live_progress(&self) -> Option<AnkiLiveProgress> {
        self.anki_live.as_ref().map(|live| live.progress)
    }
    pub(super) fn anki_operation_busy(&self) -> bool {
        self.anki_live
            .as_ref()
            .is_some_and(|live| live.progress != AnkiLiveProgress::Finished)
    }
    pub fn take_anki_live_invocation(&mut self) -> Option<AnkiLiveInvocation> {
        let live = self.anki_live.as_mut()?;
        if live.progress != AnkiLiveProgress::Ready {
            return None;
        }
        live.progress = AnkiLiveProgress::Invoking;
        Some(AnkiLiveInvocation::new(live.request.clone()))
    }
}
impl Yomoku {
    pub(super) fn begin_anki_intent(
        &self,
        model: &mut Model,
        intent: AnkiLiveIntent,
    ) -> Command<Effect, Event> {
        if model.anki_live.is_some() {
            return Command::done();
        }
        model.anki_live = Some(AnkiLiveState {
            request: AnkiLiveRequest::new(intent),
            progress: AnkiLiveProgress::Ready,
            callback: None,
        });
        Command::done()
    }
    pub(super) fn update_anki_live(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::BeginPreparedAnkiExport {
                preparation_id,
                expected_phase,
            } => {
                if !model.anki_export_view().is_some_and(|v| {
                    v.preparation_id == *preparation_id && v.phase == *expected_phase
                }) {
                    return Some(Command::done());
                }
                return self.update_anki_live(&Event::BeginAnkiLive, model);
            }
            Event::DispatchAnkiLive => {
                let (surface, reading, deck_id) =
                    if let Some(export) = model.prepared_anki_export.as_ref() {
                        (
                            export.note().surface().as_str().to_owned(),
                            export.note().reading().to_owned(),
                            export.deck().into(),
                        )
                    } else if let Some(deck) = model.anki_setup.live_deck {
                        (String::new(), String::new(), deck)
                    } else {
                        return Some(Command::done());
                    };
                let Some(invocation) = model.take_anki_live_invocation() else {
                    return Some(Command::done());
                };
                return Some(
                    Command::request_from_shell(invocation.operation(surface, reading, deck_id))
                        .then_send(move |result| {
                            Event::FinishAnkiLive(Box::new(invocation.complete_provider(result)))
                        }),
                );
            }
            Event::BeginAnkiLive => {
                if !model.anki_export_gate_open() {
                    return Some(Command::done());
                }
                let Some(export) = model.prepared_anki_export.as_ref() else {
                    return Some(Command::done());
                };
                let Some(preflight) = &model.anki_export_preflight else {
                    return Some(Command::done());
                };
                if preflight.request.export() != export {
                    return Some(Command::done());
                }
                let intent = match &preflight.result {
                    Some(Ok(AnkiPreflightPlan::ReuseModel(id))) => AnkiSubmission::new(
                        export.note().clone(),
                        export.deck(),
                        *id,
                        export.layout().clone(),
                    )
                    .map(|v| AnkiLiveIntent::Note(Box::new(v))),
                    Some(Ok(AnkiPreflightPlan::CreateModel)) => {
                        AnkiModelCreation::new(export.layout().clone())
                            .map(|v| AnkiLiveIntent::Model(Box::new(v)))
                    }
                    _ => return Some(Command::done()),
                };
                let Ok(intent) = intent else {
                    return Some(Command::done());
                };
                return Some(self.begin_anki_intent(model, intent));
            }
            Event::FinishAnkiLive(callback) => {
                let Some(live) = &mut model.anki_live else {
                    return Some(Command::done());
                };
                if &live.request != callback.request()
                    || live.progress != AnkiLiveProgress::Invoking
                {
                    return Some(Command::done());
                }
                if matches!(
                    callback.outcome(),
                    crate::anki::AnkiLiveOutcome::Note(crate::anki::AnkiSubmissionOutcome::Added(
                        _
                    ))
                ) {
                    if let Some(id) = model.anki_notice_sequence.checked_add(1) {
                        model.anki_notice_sequence = id;
                        model.anki_added_notice = Some(id);
                    }
                }
                live.progress = AnkiLiveProgress::Finished;
                live.callback = Some(*callback.clone());
                if model.anki_setup.live_deck.is_some() {
                    model.anki_setup.phase = match callback.outcome() {
                        crate::anki::AnkiLiveOutcome::Model(
                            crate::anki::AnkiModelCreationOutcome::Created(_),
                        ) => crate::anki::AnkiSetupPhase::Ready,
                        _ => crate::anki::AnkiSetupPhase::Failed(
                            crate::anki::AnkiExportError::ProviderFailure,
                        ),
                    };
                }
                model.complete_entry_action();
            }
            Event::AbandonAnkiLive => {
                // Invalidate the owner, never recreate an invocation from its payload.
                model.anki_live = None;
                model.prepared_anki_export = None;
                model.anki_export_preflight = None;
                model.anki_setup.live_deck = None;
                model.anki_setup.phase = crate::anki::AnkiSetupPhase::Unchecked;
                model.anki_entries.clear();
            }
            _ => return None,
        }
        Some(Command::done())
    }
}
#[cfg(test)]
mod tests;
