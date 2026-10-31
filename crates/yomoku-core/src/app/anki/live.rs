//! App-owned begin, one native permit and durable outcome acceptance. Provider
//! dispatch is deliberately a later boundary; this module performs no I/O.

use super::*;
use crate::anki::{
    AnkiLiveIntent, AnkiLiveInvocation, AnkiLiveProgress, AnkiLiveRequest, AnkiLiveState,
    AnkiModelCreation, AnkiPreflightPlan, AnkiSubmission,
};

impl Model {
    pub fn anki_live_progress(&self) -> Option<AnkiLiveProgress> {
        self.anki_live.as_ref().map(|live| live.progress)
    }

    pub(super) fn anki_live_blocks_recovery(&self) -> bool {
        self.anki_live_progress()
            .is_some_and(|progress| progress != AnkiLiveProgress::Finished)
    }

    /// Native dispatcher prerequisite, consumed once after matching successful
    /// begin. No shell call site yet. Fresh provider checks remain mandatory.
    pub fn take_anki_live_invocation(&mut self) -> Option<AnkiLiveInvocation> {
        let live = self.anki_live.as_mut()?;
        if live.progress != AnkiLiveProgress::Ready
            || self.anki_recovery_commit.is_some()
            || self.anki_recovery_commit_error.is_some()
            || self.anki_recovery_read.is_some()
            || self.anki_recovery_read_error.is_some()
            || !matches!(&self.anki_recovery, AnkiRecoveryState::Restored(pair)
                if live.begun.as_ref() == Some(pair))
        {
            return None;
        }
        live.progress = AnkiLiveProgress::Invoking;
        Some(AnkiLiveInvocation::new(live.request.clone()))
    }
}

impl Yomoku {
    pub(super) fn update_anki_live(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
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
                    .map(|value| AnkiLiveIntent::Note(Box::new(value))),
                    Some(Ok(AnkiPreflightPlan::CreateModel)) => {
                        AnkiModelCreation::new(export.layout().clone())
                            .map(|value| AnkiLiveIntent::Model(Box::new(value)))
                    }
                    // Exact duplicates and failed/incomplete reads grant no begin.
                    _ => return Some(Command::done()),
                };
                let Ok(intent) = intent else {
                    return Some(Command::done());
                };
                let request = AnkiLiveRequest::new(intent);
                model.anki_live = Some(AnkiLiveState {
                    request: request.clone(),
                    progress: AnkiLiveProgress::Beginning,
                    begun: None,
                    callback: None,
                });
                return Some(
                    Command::request_from_shell(StorageOperation::BeginAnkiLive(Box::new(
                        request.clone(),
                    )))
                    .then_send(move |result| {
                        Event::AnkiLiveBeginFinished(Box::new(request), result)
                    }),
                );
            }
            Event::AnkiLiveBeginFinished(request, result) => {
                let Some(live) = &mut model.anki_live else {
                    return Some(Command::done());
                };
                if live.request != **request || live.progress != AnkiLiveProgress::Beginning {
                    return Some(Command::done());
                }
                match result {
                    StorageResult::AnkiLiveBegun(snapshot)
                        if live.request.accepts_begin(snapshot)
                            && model.anki_recovery.allows_preparation()
                            && model.anki_recovery_read.is_none()
                            && model.anki_recovery_read_error.is_none()
                            && model.anki_recovery_commit.is_none()
                            && model.anki_recovery_commit_error.is_none() =>
                    {
                        model.anki_recovery = AnkiRecoveryState::Restored(snapshot.clone());
                        live.begun = Some(snapshot.clone());
                        live.progress = AnkiLiveProgress::Ready;
                    }
                    StorageResult::Failed(error) => {
                        live.progress = AnkiLiveProgress::Failed(*error);
                    }
                    _ => live.progress = AnkiLiveProgress::Failed(DocumentError::Unavailable),
                }
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
                if live.begun.as_ref().is_none_or(|pair| {
                    callback.expected_result(pair).is_none()
                        || model.anki_recovery != AnkiRecoveryState::Restored(pair.clone())
                }) {
                    live.progress = AnkiLiveProgress::Failed(DocumentError::Unavailable);
                    return Some(Command::done());
                }
                live.progress = AnkiLiveProgress::Saving;
                live.callback = Some(*callback.clone());
                let request = live.request.clone();
                return Some(
                    Command::request_from_shell(StorageOperation::FinishAnkiLive(callback.clone()))
                        .then_send(move |result| {
                            Event::AnkiLiveOutcomeFinished(Box::new(request), result)
                        }),
                );
            }
            Event::AnkiLiveOutcomeFinished(request, result) => {
                let Some(live) = &mut model.anki_live else {
                    return Some(Command::done());
                };
                if live.request != **request || live.progress != AnkiLiveProgress::Saving {
                    return Some(Command::done());
                }
                let expected = live
                    .callback
                    .as_ref()
                    .and_then(|callback| callback.expected_result(live.begun.as_ref()?.as_ref()));
                match result {
                    StorageResult::AnkiLiveFinished(snapshot)
                        if expected.as_ref() == Some(snapshot)
                            && matches!(&model.anki_recovery, AnkiRecoveryState::Restored(pair)
                                if live.begun.as_ref() == Some(pair)) =>
                    {
                        model.anki_recovery = AnkiRecoveryState::Restored(snapshot.clone());
                        live.progress = AnkiLiveProgress::Finished;
                    }
                    StorageResult::Failed(error) => {
                        live.progress = AnkiLiveProgress::Failed(*error);
                    }
                    _ => live.progress = AnkiLiveProgress::Failed(DocumentError::Unavailable),
                }
            }
            Event::AbandonAnkiLive => {
                if let Some(live) = &mut model.anki_live {
                    if live.progress != AnkiLiveProgress::Finished {
                        live.progress = AnkiLiveProgress::Failed(DocumentError::Unavailable);
                    }
                }
            }
            _ => return None,
        }
        Some(Command::done())
    }
}

#[cfg(test)]
mod tests;
