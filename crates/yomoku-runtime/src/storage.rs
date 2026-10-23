//! A single FIFO worker owns each session's SQLite connection. Opening/migration
//! is lazy and retryable, and never happens on the dispatching/UI thread.

use std::{path::PathBuf, sync::mpsc, thread};

use crux_core::middleware::{EffectMiddleware, EffectResolver};
use yomoku_core::{
    document::PasteText,
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{Storage, StorageError};

type Job = (StorageOperation, EffectResolver<StorageResult>);

pub struct StorageEffects {
    sender: mpsc::Sender<Job>,
}

impl StorageEffects {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel::<Job>();
        thread::Builder::new()
            .name("yomoku-storage".into())
            .spawn(move || {
                let mut storage = None;
                while let Ok((operation, mut resolver)) = receiver.recv() {
                    let output = execute(&path, &mut storage, operation)
                        .unwrap_or_else(StorageResult::Failed);
                    resolver.resolve(output);
                }
                // Closing the bridge drops the sender. Queued operations finish;
                // the connection and worker then exit without blocking Android UI.
            })?;
        Ok(Self { sender })
    }
}

impl EffectMiddleware for StorageEffects {
    type Op = StorageOperation;

    fn try_process_effect(&self, operation: Self::Op, resolver: EffectResolver<StorageResult>) {
        // The worker exits only when all senders are dropped. No detached task
        // or per-operation thread can reorder database commands.
        assert!(
            self.sender.send((operation, resolver)).is_ok(),
            "storage worker exited unexpectedly"
        );
    }
}

fn execute(
    path: &PathBuf,
    storage: &mut Option<Storage>,
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
        StorageOperation::CreatePaste(_) => storage
            .create_paste(&paste.expect("validated paste"))
            .map(StorageResult::Created)
            .map_err(map_error),
        StorageOperation::LoadPaste(id) => Ok(StorageResult::Loaded(
            storage
                .load_paste(id)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?,
            storage.page_length().map_err(map_error)?,
        )),
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
    }
}

fn map_error(error: StorageError) -> DocumentError {
    match error {
        StorageError::Sqlite(_) => DocumentError::Unavailable,
        StorageError::InvalidDocument | StorageError::InvalidCompletion => {
            DocumentError::InvalidDocument
        }
        StorageError::InvalidSettings => DocumentError::InvalidSettings,
        StorageError::UnsupportedSchemaVersion { .. }
        | StorageError::InvalidMigrationHistory
        | StorageError::InvalidVocabulary
        | StorageError::UnrecognizedDatabase => DocumentError::IncompatibleDatabase,
    }
}
