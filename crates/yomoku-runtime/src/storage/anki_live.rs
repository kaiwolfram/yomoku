//! Session-local live storage tokens never leave the SQLite FIFO worker.

use std::path::Path;

use yomoku_core::{
    anki::{AnkiLiveRequest, AnkiLiveRequestWeak},
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{anki_live::AnkiLiveAttempt, Storage};

#[derive(Default)]
pub(super) struct AnkiLiveWorker {
    retained: Option<(AnkiLiveRequest, AnkiLiveAttempt)>,
    // Keep identities of consumed requests only while a callback still owns
    // them. This rejects begin replay after completion/restoration without
    // retaining payloads or an ever-growing strong-reference journal.
    seen: Vec<AnkiLiveRequestWeak>,
}

impl AnkiLiveWorker {
    pub(super) fn execute(
        &mut self,
        path: &Path,
        storage: &mut Option<Storage>,
        operation: StorageOperation,
    ) -> Result<StorageResult, DocumentError> {
        let unavailable = DocumentError::Unavailable;
        match &operation {
            StorageOperation::BeginAnkiLive(request) => {
                self.seen.retain(AnkiLiveRequestWeak::is_live);
                if self.seen.iter().any(|seen| seen.matches(request)) {
                    return Err(unavailable);
                }
                // Even failed begin calls are consumed; a fresh app request
                // needs read-only restoration and fresh preparation/preflight.
                self.seen.push(request.downgrade());
                if self.retained.is_some() {
                    return Err(unavailable);
                }
            }
            StorageOperation::FinishAnkiLive(callback) => {
                if self.retained.as_ref().map(|(request, _)| request) != Some(callback.request()) {
                    return Err(unavailable);
                }
            }
            StorageOperation::LoadAnkiRecovery => {
                // Restoration invalidates live authority even if its read fails.
                self.retained = None;
            }
            _ => unreachable!("live worker operation"),
        }
        if storage.is_none() {
            *storage = Some(Storage::open(path).map_err(super::map_error)?);
        }
        let storage = storage.as_mut().expect("opened storage");
        match operation {
            StorageOperation::BeginAnkiLive(request) => {
                let attempt = storage
                    .begin_anki_live(request.intent())
                    .map_err(|_| unavailable)?;
                let snapshot = Box::new(attempt.snapshot().clone());
                self.retained = Some((*request, attempt));
                Ok(StorageResult::AnkiLiveBegun(snapshot))
            }
            StorageOperation::FinishAnkiLive(callback) => {
                // Consume before SQL, including wrong-kind and failed commits.
                // No repeated callback or retry can use this token again.
                let (_, attempt) = self.retained.take().expect("checked owner");
                storage
                    .finish_anki_live(&attempt, callback.outcome().clone())
                    .map(StorageResult::AnkiLiveFinished)
                    .map_err(|_| unavailable)
            }
            StorageOperation::LoadAnkiRecovery => storage
                .anki_recovery_snapshot()
                .map(|snapshot| StorageResult::AnkiRecoveryLoaded(Box::new(snapshot)))
                .map_err(|_| unavailable),
            _ => unreachable!("live worker operation"),
        }
    }
}

#[cfg(test)]
mod tests;
