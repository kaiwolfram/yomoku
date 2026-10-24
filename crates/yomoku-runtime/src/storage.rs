//! A single FIFO worker owns each session's SQLite connection. Opening/migration
//! is lazy and retryable, and never happens on the dispatching/UI thread.

use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
};

use crux_core::middleware::{EffectMiddleware, EffectResolver};
use yomoku_core::{
    document::PasteText,
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{Storage, StorageError};

use crate::imports::{AnalyzerLifecycle, AnalyzerLoader, Imports};

pub(crate) enum Job {
    Storage(StorageOperation, EffectResolver<StorageResult>),
    Foreground(bool),
}

pub struct StorageEffects {
    sender: Arc<mpsc::Sender<Job>>,
}

impl StorageEffects {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        Self::spawn(path, None)
    }

    pub fn with_analyzed_imports(
        path: PathBuf,
        loader: AnalyzerLoader,
    ) -> std::io::Result<(Self, AnalyzerLifecycle)> {
        let effects = Self::spawn(path, Some(loader))?;
        let lifecycle = AnalyzerLifecycle {
            sender: Arc::downgrade(&effects.sender),
        };
        Ok((effects, lifecycle))
    }

    fn spawn(path: PathBuf, loader: Option<AnalyzerLoader>) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("yomoku-storage".into())
            .spawn(move || {
                let mut storage = None;
                let mut imports = loader.map(Imports::new);
                while let Ok(job) = receiver.recv() {
                    match job {
                        Job::Storage(operation, mut resolver) => {
                            let output = execute(&path, &mut storage, &mut imports, operation)
                                .unwrap_or_else(StorageResult::Failed);
                            if let Some(imports) = imports.as_mut() {
                                imports.release_if_hidden();
                            }
                            resolver.resolve(output);
                        }
                        Job::Foreground(foreground) => {
                            if let Some(imports) = imports.as_mut() {
                                imports.set_foreground(foreground);
                            }
                        }
                    }
                }
                // Closing the bridge drops the sender. Queued operations finish;
                // the connection and worker then exit without blocking Android UI.
            })?;
        Ok(Self {
            sender: Arc::new(sender),
        })
    }
}

impl EffectMiddleware for StorageEffects {
    type Op = StorageOperation;

    fn try_process_effect(&self, operation: Self::Op, resolver: EffectResolver<StorageResult>) {
        // The worker exits only when all senders are dropped. No detached task
        // or per-operation thread can reorder database commands.
        assert!(
            self.sender.send(Job::Storage(operation, resolver)).is_ok(),
            "storage worker exited unexpectedly"
        );
    }
}

fn execute(
    path: &PathBuf,
    storage: &mut Option<Storage>,
    imports: &mut Option<Imports>,
    operation: StorageOperation,
) -> Result<StorageResult, DocumentError> {
    // Reject blank input before opening/creating a database.
    let paste = match &operation {
        StorageOperation::CreatePaste(text) => {
            Some(PasteText::new(text.clone()).map_err(|_| DocumentError::EmptyPaste)?)
        }
        _ => None,
    };
    if storage.is_none() {
        *storage = Some(Storage::open(path).map_err(map_error)?);
    }
    let storage = storage.as_mut().expect("opened storage");
    match operation {
        StorageOperation::List => Ok(StorageResult::Listed(
            storage.list_documents().map_err(map_error)?,
            storage.page_length().map_err(map_error)?,
        )),
        StorageOperation::CreatePaste(_) => {
            let paste = paste.expect("validated paste");
            if let Some(imports) = imports.as_mut() {
                imports.import(storage, &paste)
            } else {
                // Low-level storage fixtures only. NativeBridge always supplies
                // an analyzed loader and cannot select this text-only branch.
                storage
                    .create_paste(&paste)
                    .map(StorageResult::Created)
                    .map_err(map_error)
            }
        }
        StorageOperation::LoadPaste(id) => storage
            .load_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::SavePageLength(length) => storage
            .save_page_length(length)
            .map(StorageResult::PageLengthSaved)
            .map_err(map_error),
        StorageOperation::SaveReadingPosition {
            document_id,
            position,
        } => storage
            .save_reading_position(document_id, position)
            .map_err(map_error)?
            .map(StorageResult::PositionSaved)
            .ok_or(DocumentError::NotFound),
        StorageOperation::LoadReadingTime {
            document_id,
            position,
        } => storage
            .load_pending_reading_time(document_id, position)
            .map(StorageResult::ReadingTimeLoaded)
            .map_err(map_error),
        StorageOperation::SaveReadingTime {
            previous,
            checkpoint,
        } => {
            if previous.value.document_id != checkpoint.document_id
                || previous.position() != checkpoint.position
            {
                return Err(DocumentError::InvalidDocument);
            }
            storage
                .save_pending_reading_time(previous, checkpoint.pending)
                .map(StorageResult::ReadingTimeSaved)
                .map_err(map_error)
        }
        StorageOperation::PreparePageCompletion(intent) => storage
            .prepare_page_completion(intent)
            .map(StorageResult::PageCompletionPrepared)
            .map_err(map_error),
        StorageOperation::CompletePage(prepared) => {
            let outcome = storage
                .record_analyzed_page_completion(prepared.completion, prepared.analysis_id)
                .map_err(map_error)?;
            // If this read fails, retry the frozen command. Repository replay
            // has no side effects and returns no duplicate status notification.
            let document = storage
                .load_paste(prepared.completion.document_id)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?;
            Ok(StorageResult::PageCompleted(document, outcome))
        }
    }
}

pub(crate) fn map_error(error: StorageError) -> DocumentError {
    match error {
        StorageError::Sqlite(_) => DocumentError::Unavailable,
        StorageError::InvalidDocument
        | StorageError::InvalidCompletion
        | StorageError::InvalidReadingTime => DocumentError::InvalidDocument,
        StorageError::InvalidSettings => DocumentError::InvalidSettings,
        StorageError::UnsupportedSchemaVersion { .. }
        | StorageError::InvalidMigrationHistory
        | StorageError::InvalidVocabulary
        | StorageError::InvalidAnalysis
        | StorageError::UnrecognizedDatabase => DocumentError::IncompatibleDatabase,
    }
}
