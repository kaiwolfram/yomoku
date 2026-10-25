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
    reader::ReaderVocabulary,
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{
    dictionary::Dictionary, reader_words::ReaderWordError, ImportCancellation, Storage,
    StorageError,
};

/// Worker-only publisher/open callback. Failed loads remain retryable; the
/// returned path must refer to a verified immutable dictionary index.
pub type WordDictionaryLoader = Box<dyn FnMut() -> Result<PathBuf, DocumentError> + Send>;

use crate::imports::{AnalyzerLifecycle, AnalyzerLoader, Imports, TxtImportControl};

pub(crate) enum Job {
    Storage(
        StorageOperation,
        EffectResolver<StorageResult>,
        Option<Arc<ImportCancellation>>,
    ),
    Foreground(bool),
}

pub struct StorageEffects {
    sender: Arc<mpsc::Sender<Job>>,
    txt: Arc<TxtImportControl>,
}

impl StorageEffects {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        Self::spawn(path, None, None)
    }

    pub fn with_analyzed_imports(
        path: PathBuf,
        loader: AnalyzerLoader,
    ) -> std::io::Result<(Self, AnalyzerLifecycle)> {
        let effects = Self::spawn(path, Some(loader), None)?;
        let lifecycle = AnalyzerLifecycle {
            sender: Arc::downgrade(&effects.sender),
            txt: Arc::downgrade(&effects.txt),
        };
        Ok((effects, lifecycle))
    }

    pub fn with_dictionaries(
        path: PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
    ) -> std::io::Result<(Self, AnalyzerLifecycle)> {
        let effects = Self::spawn(path, Some(loader), Some(word_loader))?;
        let lifecycle = AnalyzerLifecycle {
            sender: Arc::downgrade(&effects.sender),
            txt: Arc::downgrade(&effects.txt),
        };
        Ok((effects, lifecycle))
    }

    fn spawn(
        path: PathBuf,
        loader: Option<AnalyzerLoader>,
        mut word_loader: Option<WordDictionaryLoader>,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let txt = Arc::new(TxtImportControl::default());
        let worker_txt = txt.clone();
        thread::Builder::new()
            .name("yomoku-storage".into())
            .spawn(move || {
                let mut storage = None;
                let mut dictionary = None;
                let mut imports = loader.map(Imports::new);
                while let Ok(job) = receiver.recv() {
                    match job {
                        Job::Storage(operation, mut resolver, cancellation) => {
                            let output = if cancellation
                                .as_ref()
                                .is_some_and(|value| value.is_cancelled())
                            {
                                Ok(StorageResult::ImportCancelled)
                            } else {
                                execute(
                                    &path,
                                    &mut storage,
                                    &mut imports,
                                    &mut word_loader,
                                    &mut dictionary,
                                    operation,
                                    cancellation.as_deref(),
                                )
                            }
                            .unwrap_or_else(StorageResult::Failed);
                            if let Some(cancellation) = &cancellation {
                                worker_txt.finish(cancellation);
                            }
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
            txt,
        })
    }
}

impl EffectMiddleware for StorageEffects {
    type Op = StorageOperation;

    fn try_process_effect(&self, operation: Self::Op, resolver: EffectResolver<StorageResult>) {
        // The core accepts one storage operation at a time. Publish its fresh
        // attempt before enqueue; a busy/rejected core event never replaces it.
        let cancellation =
            matches!(operation, StorageOperation::ImportTxt(_)).then(|| self.txt.begin());
        // The worker exits only when all senders are dropped. No detached task
        // or per-operation thread can reorder database commands.
        assert!(
            self.sender
                .send(Job::Storage(operation, resolver, cancellation))
                .is_ok(),
            "storage worker exited unexpectedly"
        );
    }
}

fn execute(
    path: &PathBuf,
    storage: &mut Option<Storage>,
    imports: &mut Option<Imports>,
    word_loader: &mut Option<WordDictionaryLoader>,
    dictionary: &mut Option<Dictionary>,
    operation: StorageOperation,
    cancellation: Option<&ImportCancellation>,
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
        StorageOperation::ImportTxt(input) => imports
            .as_mut()
            .ok_or(DocumentError::Unavailable)?
            .import_txt(
                storage,
                &input,
                cancellation.expect("TXT attempt published before enqueue"),
            ),
        StorageOperation::LoadPaste(id) => storage
            .load_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::OpenReader(id) => storage
            .open_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::LoadReaderTokens(key) => storage
            .load_reader_tokens(key.document_id, key.page)
            .map_err(map_error)?
            .map(StorageResult::ReaderTokensLoaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::LoadReaderWord(selection) => {
            if dictionary.is_none() {
                let path = word_loader.as_mut().ok_or(DocumentError::Unavailable)?()?;
                *dictionary = Some(Dictionary::open(path).map_err(|_| DocumentError::Unavailable)?);
            }
            let result = storage.load_reader_word(
                dictionary.as_ref().expect("opened dictionary"),
                selection.key,
                selection.token_index,
            );
            match result {
                Ok(Some(details)) => Ok(StorageResult::ReaderWordLoaded(Box::new(details))),
                Ok(None) => Err(DocumentError::NotFound),
                Err(ReaderWordError::Storage(error)) => Err(map_error(error)),
                Err(ReaderWordError::InvalidSelection) => Err(DocumentError::InvalidDocument),
                Err(ReaderWordError::Dictionary(_)) => {
                    // A repaired atomic publication must be reopened on explicit retry.
                    *dictionary = None;
                    Err(DocumentError::Unavailable)
                }
            }
        }
        StorageOperation::SetReaderStatus(change) => storage
            .set_vocabulary_status(
                change.vocabulary_id,
                change.status,
                change.occurred_at_unix_ms,
            )
            .map(|entry| {
                StorageResult::ReaderStatusChanged(
                    change,
                    entry.map(|entry| ReaderVocabulary {
                        id: entry.id,
                        status: entry.status,
                    }),
                )
            })
            .map_err(map_error),
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
