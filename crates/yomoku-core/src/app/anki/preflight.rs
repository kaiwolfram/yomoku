//! App-owned preflight lifecycle, independent of the local reader's I/O slot.

use super::*;
use crate::anki::{AnkiExportError, AnkiPreflight, AnkiPreflightPlan, AnkiPreflightRequest};

impl Model {
    pub fn pending_anki_export_preflight(&self) -> Option<&AnkiPreflightRequest> {
        let preflight = self.anki_export_preflight.as_ref()?;
        preflight.result.is_none().then_some(&preflight.request)
    }

    /// An observation only. Any future durable begin must check the journal
    /// gates again; a provider can change after this read completes.
    pub fn anki_export_preflight_result(
        &self,
    ) -> Option<&Result<AnkiPreflightPlan, AnkiExportError>> {
        self.anki_export_preflight.as_ref()?.result.as_ref()
    }

    pub(super) fn anki_export_gate_open(&self) -> bool {
        self.anki_recovery.allows_preparation()
            && self.anki_recovery_commit.is_none()
            && self.anki_recovery_commit_error.is_none()
            && self.anki_recovery_read.is_none()
            && self.anki_recovery_read_error.is_none()
    }
}

impl Yomoku {
    pub(super) fn update_anki_preflight(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        match event {
            Event::BeginAnkiExportPreflight | Event::RetryAnkiExportPreflight => {
                if !model.anki_export_gate_open()
                    || (matches!(event, Event::BeginAnkiExportPreflight)
                        && model.anki_export_preflight.is_some())
                    || (matches!(event, Event::RetryAnkiExportPreflight)
                        && model.anki_export_preflight.is_none())
                {
                    return Some(Command::done());
                }
                if let Some(export) = &model.prepared_anki_export {
                    model.anki_export_preflight = Some(AnkiPreflight {
                        request: AnkiPreflightRequest::new(export.clone()),
                        result: None,
                    });
                }
            }
            Event::AnkiExportPreflightFinished(request, observation) => {
                if model.pending_anki_export_preflight() != Some(request.as_ref()) {
                    return Some(Command::done());
                }
                let result = if !model.anki_export_gate_open()
                    || model.prepared_anki_export.as_ref() != Some(request.export())
                {
                    Err(AnkiExportError::InvalidResponse)
                } else {
                    observation
                        .clone()
                        .and_then(|value| request.evaluate(value))
                };
                model
                    .anki_export_preflight
                    .as_mut()
                    .expect("active read")
                    .result = Some(result);
            }
            _ => return None,
        }
        // This bounded lifecycle grants no live token and emits no provider or
        // storage effect. Typed dispatch and durable begin follow separately.
        Some(Command::done())
    }
}

#[cfg(test)]
mod tests;
