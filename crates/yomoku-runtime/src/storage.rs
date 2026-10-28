//! A single FIFO worker owns each session's SQLite connection. Opening/migration
//! is lazy and retryable, and never happens on the dispatching/UI thread.

use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
};

use crux_core::middleware::{EffectMiddleware, EffectResolver};
use yomoku_core::{
    document::{DocumentId, PasteText},
    reader::ReaderVocabulary,
    reader_images::ReaderImageAsset,
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{
    custom_dictionary::CustomDictionaryError, dictionary::Dictionary,
    reader_words::ReaderWordError, ImportCancellation, Storage, StorageError,
};

/// Worker-only publisher/open callback. Failed loads remain retryable; the
/// returned path must refer to a verified immutable dictionary index.
pub type WordDictionaryLoader = Box<dyn FnMut() -> Result<PathBuf, DocumentError> + Send>;

use crate::imports::{AnalyzerLifecycle, AnalyzerLoader, FileImportControl, Imports};

mod effective;
#[cfg(test)]
mod effective_tests;

pub(crate) enum Job {
    EpubCover(DocumentId, crate::epub_assets::CoverCallback),
    EpubAsset(
        DocumentId,
        ReaderImageAsset,
        crate::epub_assets::AssetCallback,
    ),
    Storage(
        StorageOperation,
        EffectResolver<StorageResult>,
        Option<Arc<ImportCancellation>>,
    ),
    Foreground(bool),
}

pub struct StorageEffects {
    sender: Arc<mpsc::Sender<Job>>,
    file: Arc<FileImportControl>,
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
            file: Arc::downgrade(&effects.file),
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
            file: Arc::downgrade(&effects.file),
        };
        Ok((effects, lifecycle))
    }

    fn spawn(
        path: PathBuf,
        loader: Option<AnalyzerLoader>,
        mut word_loader: Option<WordDictionaryLoader>,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let file = Arc::new(FileImportControl::default());
        let worker_file = file.clone();
        thread::Builder::new()
            .name("yomoku-storage".into())
            .spawn(move || {
                let mut storage = None;
                let mut dictionary = None;
                let mut imports = loader.map(Imports::new);
                while let Ok(job) = receiver.recv() {
                    match job {
                        Job::EpubCover(document_id, callback) => {
                            let result = (|| {
                                if storage.is_none() {
                                    storage = Some(Storage::open(&path).map_err(map_error)?);
                                }
                                storage
                                    .as_mut()
                                    .expect("opened storage")
                                    .load_epub_cover_reference(document_id)
                                    .map_err(map_error)
                            })();
                            callback(result);
                        }
                        Job::EpubAsset(document_id, asset, callback) => {
                            let result = (|| {
                                asset
                                    .validate()
                                    .map_err(|_| DocumentError::InvalidDocument)?;
                                if storage.is_none() {
                                    storage = Some(Storage::open(&path).map_err(map_error)?);
                                }
                                storage
                                    .as_mut()
                                    .expect("opened storage")
                                    .load_epub_image_asset(document_id, &asset)
                                    .map_err(map_error)?
                                    .ok_or(DocumentError::NotFound)
                            })();
                            callback(result);
                        }
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
                                worker_file.finish(cancellation);
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
            file,
        })
    }
}

impl EffectMiddleware for StorageEffects {
    type Op = StorageOperation;

    fn try_process_effect(&self, operation: Self::Op, resolver: EffectResolver<StorageResult>) {
        // The core accepts one storage operation at a time. Publish its fresh
        // attempt before enqueue; a busy/rejected core event never replaces it.
        let cancellation = matches!(
            operation,
            StorageOperation::ImportTxt(_)
                | StorageOperation::ImportSrt(_)
                | StorageOperation::ImportEpub(_)
        )
        .then(|| self.file.begin());
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
        StorageOperation::Effective(operation) => {
            effective::execute(storage, imports, dictionary, word_loader, operation)
                .map(StorageResult::Effective)
        }
        StorageOperation::List => Ok(StorageResult::Listed(
            storage.list_documents().map_err(map_error)?,
            storage.page_length().map_err(map_error)?,
            storage.reader_typography().map_err(map_error)?,
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
                    .map_err(map_error)?
                    .library_summary()
                    .map(StorageResult::Created)
                    .map_err(|_| DocumentError::InvalidDocument)
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
        StorageOperation::ImportSrt(input) => imports
            .as_mut()
            .ok_or(DocumentError::Unavailable)?
            .import_srt(
                storage,
                &input,
                cancellation.expect("SRT attempt published before enqueue"),
            ),
        StorageOperation::ImportEpub(input) => imports
            .as_mut()
            .ok_or(DocumentError::Unavailable)?
            .import_epub(
                storage,
                &input,
                cancellation.expect("EPUB attempt published before enqueue"),
            ),
        StorageOperation::LoadPaste(id) => storage
            .load_document_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::OpenReader(id) => storage
            .open_document_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::LoadReaderTokens(key) => storage
            .load_reader_tokens(key.document_id, key.page)
            .map_err(map_error)?
            .map(StorageResult::ReaderTokensLoaded)
            .ok_or(DocumentError::NotFound),
        StorageOperation::LoadReaderWord(selection) => {
            let result = storage.load_reader_word(
                word_dictionary(dictionary, word_loader)?,
                selection.key,
                selection.token_index,
            );
            result
                .map_err(|error| map_word_error(error, dictionary))?
                .map(|details| StorageResult::ReaderWordLoaded(Box::new(details)))
                .ok_or(DocumentError::NotFound)
        }
        StorageOperation::SaveReaderCustomEntry {
            selection,
            english_meaning,
            reading,
        } => {
            let result = storage.save_reader_custom_entry(
                word_dictionary(dictionary, word_loader)?,
                selection.key,
                selection.token_index,
                english_meaning,
                reading,
            );
            let entry = result
                .map_err(|error| map_custom_error(error, dictionary))?
                .ok_or(DocumentError::NotFound)?;
            Ok(StorageResult::ReaderCustomEntryChanged(
                selection,
                Some(entry.into()),
            ))
        }
        StorageOperation::DeleteReaderCustomEntry(selection) => {
            let result = storage.delete_reader_custom_entry(
                word_dictionary(dictionary, word_loader)?,
                selection.key,
                selection.token_index,
            );
            result.map_err(|error| map_custom_error(error, dictionary))?;
            Ok(StorageResult::ReaderCustomEntryChanged(selection, None))
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
        StorageOperation::SaveReaderTypography(typography) => storage
            .save_reader_typography(typography)
            .map(StorageResult::ReaderTypographySaved)
            .map_err(map_error),
        StorageOperation::SavePageLength(length) => storage
            .save_page_length(length)
            .map(StorageResult::PageLengthSaved)
            .map_err(map_error),
        StorageOperation::SaveReadingPosition {
            document_id,
            position,
        } => storage
            .save_document_reading_position(document_id, position)
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
                .load_document_reader_snapshot(prepared.completion.document_id)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?
                .document;
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

fn word_dictionary<'a>(
    dictionary: &'a mut Option<Dictionary>,
    loader: &mut Option<WordDictionaryLoader>,
) -> Result<&'a Dictionary, DocumentError> {
    if dictionary.is_none() {
        let path = loader.as_mut().ok_or(DocumentError::Unavailable)?()?;
        *dictionary = Some(Dictionary::open(path).map_err(|_| DocumentError::Unavailable)?);
    }
    Ok(dictionary.as_ref().expect("opened dictionary"))
}

fn map_word_error(error: ReaderWordError, dictionary: &mut Option<Dictionary>) -> DocumentError {
    match error {
        ReaderWordError::Storage(error) => map_error(error),
        ReaderWordError::InvalidSelection | ReaderWordError::InvalidCustomEntry => {
            DocumentError::InvalidDocument
        }
        ReaderWordError::Dictionary(_) => {
            // Reopen an atomically repaired index only on explicit retry.
            *dictionary = None;
            DocumentError::Unavailable
        }
    }
}

fn map_custom_error(
    error: CustomDictionaryError,
    dictionary: &mut Option<Dictionary>,
) -> DocumentError {
    match error {
        CustomDictionaryError::Storage(error) => map_error(error),
        CustomDictionaryError::Selection(error) => map_word_error(error, dictionary),
        CustomDictionaryError::BuiltInMatch | CustomDictionaryError::InvalidEntry => {
            DocumentError::InvalidDocument
        }
    }
}
