//! Opt-in, worker-local Vocabulary read diagnostics. No surfaces, query text,
//! IDs or database paths are retained. Durations are not Android/JNI timings.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use yomoku_core::storage::StorageOperation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageRead {
    VocabularyPage,
}

#[derive(Clone, Debug)]
pub struct StorageMeasurement {
    pub operation: StorageRead,
    pub failed: bool,
    /// Before channel send to the worker's execution boundary; includes send
    /// overhead and worker scheduling, not just time behind earlier jobs.
    pub queue: Duration,
    /// Production repository call, including lazy open/migration when needed,
    /// result/error mapping and worker cleanup. Not SQL stepping alone.
    pub execute: Duration,
    /// Resolver entry through return: core response update, effect routing,
    /// request serialization and synchronous host callback. Excludes view().
    pub resolve: Duration,
}

/// Runs after resolution on the storage worker. Keep it nonblocking; do not
/// reenter the bridge. Observer cost is excluded from the reported durations
/// but can delay later jobs. Callback entry precedes measurement delivery.
pub type StorageObserver = Arc<dyn Fn(StorageMeasurement) + Send + Sync>;

pub(crate) struct QueuedMeasurement {
    observer: StorageObserver,
    operation: StorageRead,
    enqueued: Instant,
}

impl QueuedMeasurement {
    pub(crate) fn new(
        observer: Option<&StorageObserver>,
        operation: &StorageOperation,
    ) -> Option<Box<Self>> {
        let observer = observer?;
        let operation = match operation {
            StorageOperation::LoadVocabulary { .. } => StorageRead::VocabularyPage,
            _ => return None,
        };
        Some(Box::new(Self {
            observer: Arc::clone(observer),
            operation,
            enqueued: Instant::now(),
        }))
    }

    pub(crate) fn finish(
        self,
        started: Instant,
        executed: Instant,
        resolved: Instant,
        failed: bool,
    ) {
        (self.observer)(StorageMeasurement {
            operation: self.operation,
            failed,
            queue: started.duration_since(self.enqueued),
            execute: executed.duration_since(started),
            resolve: resolved.duration_since(executed),
        });
    }
}
