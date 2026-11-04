use super::*;
use crate::anki::{
    AnkiExportPhase, AnkiExportView, AnkiLiveOutcome, AnkiLiveProgress, AnkiModelCreationOutcome,
    AnkiPreflightPlan, AnkiSubmissionOutcome,
};

impl Model {
    pub(in crate::app) fn anki_export_view(&self) -> Option<AnkiExportView> {
        let export = self.prepared_anki_export.as_ref();
        if export.is_none() && (self.anki_setup.live_deck.is_none() || self.anki_live.is_none()) {
            return None;
        }
        let phase = if let Some(live) = &self.anki_live {
            match live.progress {
                AnkiLiveProgress::Ready => AnkiExportPhase::Ready,
                AnkiLiveProgress::Invoking => AnkiExportPhase::Invoking,
                // Terminal results belong to the consumed native invocation.
                AnkiLiveProgress::Finished => match live.callback.as_ref()?.outcome() {
                    AnkiLiveOutcome::Note(outcome) => match outcome {
                        AnkiSubmissionOutcome::Added(id) => {
                            AnkiExportPhase::NoteAdded((*id).into())
                        }
                        AnkiSubmissionOutcome::Existing(notes) => AnkiExportPhase::Existing(
                            notes.ids().iter().copied().map(Into::into).collect(),
                        ),
                        AnkiSubmissionOutcome::NotInserted => AnkiExportPhase::NotSubmitted,
                        AnkiSubmissionOutcome::Uncertain { known_note_id } => {
                            AnkiExportPhase::NoteUncertain(known_note_id.map(Into::into))
                        }
                    },
                    AnkiLiveOutcome::Model(outcome) => match outcome {
                        AnkiModelCreationOutcome::Created(id) => {
                            AnkiExportPhase::ModelCreated((*id).into())
                        }
                        AnkiModelCreationOutcome::NotCreated => AnkiExportPhase::NotSubmitted,
                        AnkiModelCreationOutcome::Uncertain { known_model_id } => {
                            AnkiExportPhase::ModelUncertain(known_model_id.map(Into::into))
                        }
                    },
                },
            }
        } else if let Some(preflight) = &self.anki_export_preflight {
            match &preflight.result {
                None => AnkiExportPhase::Checking,
                Some(Err(error)) => AnkiExportPhase::ProviderFailed(*error),
                Some(Ok(AnkiPreflightPlan::CreateModel)) => AnkiExportPhase::CreateModel,
                Some(Ok(AnkiPreflightPlan::ReuseModel(id))) => {
                    AnkiExportPhase::ReuseModel((*id).into())
                }
                Some(Ok(AnkiPreflightPlan::Existing(notes))) => {
                    AnkiExportPhase::Existing(notes.ids().iter().copied().map(Into::into).collect())
                }
            }
        } else {
            AnkiExportPhase::Prepared
        };
        Some(AnkiExportView {
            preparation_id: self.anki_preparation_sequence,
            surface: export.map_or_else(String::new, |e| e.note().surface().as_str().to_owned()),
            deck_id: export.map_or_else(|| self.anki_setup.live_deck.unwrap(), |e| e.deck().into()),
            phase,
        })
    }
}
