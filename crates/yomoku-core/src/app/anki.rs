//! Startup recovery owns its own native read slot. It neither blocks local
//! reader work nor shares/clears the reader's storage_busy/error/retry state.

use super::*;
use crate::anki::AnkiRecoveryState;

mod preparation;
mod reconciliation;

impl Model {
    pub fn anki_recovery(&self) -> &AnkiRecoveryState {
        &self.anki_recovery
    }
}

impl Yomoku {
    pub(super) fn update_anki_recovery(
        &self,
        event: &Event,
        model: &mut Model,
    ) -> Option<Command<Effect, Event>> {
        if let Some(command) = self.update_anki_preparation(event, model) {
            return Some(command);
        }
        if let Some(command) = self.update_anki_reconciliation(event, model) {
            return Some(command);
        }
        match event {
            Event::RestoreAnkiRecovery if model.anki_recovery == AnkiRecoveryState::Unrestored => {}
            Event::RetryAnkiRecovery
                if model.anki_recovery_commit.is_none()
                    && (matches!(model.anki_recovery, AnkiRecoveryState::Failed(_))
                        || model.anki_recovery_commit_error.is_some()) => {}
            Event::RestoreAnkiRecovery | Event::RetryAnkiRecovery => {
                return Some(Command::done());
            }
            Event::AnkiRecoveryFinished(id, result) => {
                if model.anki_recovery_request != Some(*id) {
                    return Some(Command::done());
                }
                model.anki_recovery_request = None;
                model.anki_recovery = match result {
                    StorageResult::AnkiRecoveryLoaded(snapshot) => {
                        AnkiRecoveryState::Restored(snapshot.clone())
                    }
                    StorageResult::Failed(error) => AnkiRecoveryState::Failed(*error),
                    _ => AnkiRecoveryState::Failed(DocumentError::Unavailable),
                };
                return Some(Command::done());
            }
            _ => return None,
        }
        let Some(id) = model.anki_recovery_sequence.checked_add(1) else {
            model.anki_recovery = AnkiRecoveryState::Failed(DocumentError::Unavailable);
            return Some(Command::done());
        };
        model.anki_recovery_sequence = id;
        model.anki_recovery_request = Some(id);
        model.anki_recovery_commit_error = None;
        model.anki_recovery = AnkiRecoveryState::Loading;
        Some(
            Command::request_from_shell(StorageOperation::LoadAnkiRecovery)
                .then_send(move |result| Event::AnkiRecoveryFinished(id, result)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anki::{
        AnkiModelCreation, AnkiModelCreationId, AnkiModelLayout, AnkiModelRecovery,
        AnkiRecoverySnapshot,
    };

    fn begin(model: &mut Model, event: Event) -> Command<Effect, Event> {
        let mut command = Yomoku.update(event, model);
        let effects: Vec<_> = command.effects().collect();
        assert_eq!(effects.len(), 1);
        assert!(
            matches!(&effects[0], Effect::Storage(request) if request.operation == StorageOperation::LoadAnkiRecovery)
        );
        command
    }

    fn finish(model: &mut Model, id: u64, result: StorageResult) {
        let mut command = Yomoku.update(Event::AnkiRecoveryFinished(id, result), model);
        assert!(command.effects().next().is_none());
    }

    #[test]
    fn anki_startup_coalesces_and_only_successful_empty_restore_allows_preparation() {
        let mut model = Model::default();
        assert!(!model.anki_recovery().allows_preparation());
        let _command = begin(&mut model, Event::RestoreAnkiRecovery);
        assert_eq!(model.anki_recovery(), &AnkiRecoveryState::Loading);
        for event in [Event::RestoreAnkiRecovery, Event::RetryAnkiRecovery] {
            assert!(Yomoku.update(event, &mut model).effects().next().is_none());
        }
        assert!(!model.anki_recovery().allows_preparation());
        finish(
            &mut model,
            1,
            StorageResult::AnkiRecoveryLoaded(Box::default()),
        );
        assert!(model.anki_recovery().allows_preparation());
        // A late duplicate cannot replace the successful snapshot with an error.
        finish(
            &mut model,
            1,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(model.anki_recovery().allows_preparation());
        assert!(Yomoku
            .update(Event::RestoreAnkiRecovery, &mut model)
            .effects()
            .next()
            .is_none());
    }

    #[test]
    fn anki_startup_errors_require_explicit_retry_and_reject_old_callbacks() {
        let mut model = Model::default();
        let _first = begin(&mut model, Event::RestoreAnkiRecovery);
        finish(
            &mut model,
            1,
            StorageResult::Failed(DocumentError::IncompatibleDatabase),
        );
        assert_eq!(
            model.anki_recovery(),
            &AnkiRecoveryState::Failed(DocumentError::IncompatibleDatabase)
        );
        assert!(!model.anki_recovery().allows_preparation());
        assert!(Yomoku
            .update(Event::RestoreAnkiRecovery, &mut model)
            .effects()
            .next()
            .is_none());
        let _second = begin(&mut model, Event::RetryAnkiRecovery);
        finish(
            &mut model,
            1,
            StorageResult::AnkiRecoveryLoaded(Box::default()),
        );
        assert_eq!(model.anki_recovery(), &AnkiRecoveryState::Loading);
        finish(
            &mut model,
            2,
            StorageResult::AnkiRecoveryLoaded(Box::default()),
        );
        assert!(model.anki_recovery().allows_preparation());
    }

    #[test]
    fn anki_startup_unexpected_result_cannot_become_empty_and_exhaustion_stays_blocked() {
        let mut model = Model::default();
        let _command = begin(&mut model, Event::RestoreAnkiRecovery);
        finish(&mut model, 1, StorageResult::ImportCancelled);
        assert_eq!(
            model.anki_recovery(),
            &AnkiRecoveryState::Failed(DocumentError::Unavailable)
        );
        model.anki_recovery_sequence = u64::MAX;
        assert!(Yomoku
            .update(Event::RetryAnkiRecovery, &mut model)
            .effects()
            .next()
            .is_none());
        assert!(!model.anki_recovery().allows_preparation());
        assert_eq!(model.anki_recovery_request, None);
    }

    #[test]
    fn anki_startup_retains_old_model_layout_and_known_id_without_commands() {
        let mut layout = AnkiModelLayout::yomoku();
        layout.css.push_str("\n/* frozen 𠮷 */");
        let snapshot = Box::new(AnkiRecoverySnapshot {
            note: None,
            model: Some(AnkiModelRecovery {
                id: AnkiModelCreationId::try_from(i64::MAX).unwrap(),
                creation: AnkiModelCreation::new(layout).unwrap(),
                known_model_id: Some(crate::anki::AnkiModelId::try_from(9007199254740993).unwrap()),
            }),
        });
        let mut model = Model::with_effective_reader();
        let _command = begin(&mut model, Event::RestoreAnkiRecovery);
        finish(
            &mut model,
            1,
            StorageResult::AnkiRecoveryLoaded(snapshot.clone()),
        );
        assert_eq!(
            model.anki_recovery(),
            &AnkiRecoveryState::Restored(snapshot)
        );
        assert!(!model.anki_recovery().allows_preparation());
        for event in [Event::RetryAnkiRecovery, Event::RestoreAnkiRecovery] {
            assert!(Yomoku.update(event, &mut model).effects().next().is_none());
        }
    }

    #[test]
    fn anki_startup_does_not_release_reader_busy_or_replace_reader_errors() {
        let mut model = Model {
            storage_busy: true,
            document_error: Some(DocumentError::MissingAnalysis),
            ..Model::default()
        };
        let before = Yomoku.view(&model);
        let _command = begin(&mut model, Event::RestoreAnkiRecovery);
        finish(
            &mut model,
            1,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(Yomoku.view(&model), before);
        let _retry = begin(&mut model, Event::RetryAnkiRecovery);
        finish(
            &mut model,
            2,
            StorageResult::AnkiRecoveryLoaded(Box::default()),
        );
        assert_eq!(Yomoku.view(&model), before);
    }

    #[test]
    fn anki_startup_uses_the_exact_native_callback_and_restart_starts_closed() {
        let mut model = Model::default();
        let mut command = Yomoku.update(Event::RestoreAnkiRecovery, &mut model);
        let Effect::Storage(mut request) = command.effects().next().unwrap() else {
            panic!("native storage")
        };
        request
            .resolve(StorageResult::AnkiRecoveryLoaded(Box::default()))
            .unwrap();
        let event = command.events().next().unwrap();
        assert!(matches!(event, Event::AnkiRecoveryFinished(1, _)));
        assert!(Yomoku.update(event, &mut model).effects().next().is_none());
        assert!(model.anki_recovery().allows_preparation());
        let restarted = Model::default();
        assert_eq!(restarted.anki_recovery(), &AnkiRecoveryState::Unrestored);
        assert!(!restarted.anki_recovery().allows_preparation());
    }
}
