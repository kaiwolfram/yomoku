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
    reader_images::ReaderImageAsset,
    storage::{DocumentError, StorageOperation, StorageResult},
};
use yomoku_storage::{
    dictionary::Dictionary, reader_words::ReaderWordError, ImportCancellation, Storage,
    StorageError,
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
    Storage {
        operation: StorageOperation,
        resolver: EffectResolver<StorageResult>,
        cancellation: Option<Arc<ImportCancellation>>,
        #[cfg(feature = "storage-measurements")]
        measurement: Option<Box<crate::storage_measurements::QueuedMeasurement>>,
    },
    Foreground(bool),
}

pub struct StorageEffects {
    sender: Arc<mpsc::Sender<Job>>,
    file: Arc<FileImportControl>,
    #[cfg(feature = "storage-measurements")]
    pub(crate) measurements: Option<crate::storage_measurements::StorageObserver>,
}

impl StorageEffects {
    pub fn new(
        path: PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
    ) -> std::io::Result<(Self, AnalyzerLifecycle)> {
        let effects = Self::spawn(path, loader, word_loader)?;
        let lifecycle = AnalyzerLifecycle {
            sender: Arc::downgrade(&effects.sender),
            file: Arc::downgrade(&effects.file),
        };
        Ok((effects, lifecycle))
    }

    fn spawn(
        path: PathBuf,
        loader: AnalyzerLoader,
        mut word_loader: WordDictionaryLoader,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel::<Job>();
        let file = Arc::new(FileImportControl::default());
        let worker_file = file.clone();
        thread::Builder::new()
            .name("yomoku-storage".into())
            .spawn(move || {
                let mut storage = None;
                let mut dictionary = None;
                let mut imports = Imports::new(loader);
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
                        Job::Storage {
                            operation,
                            mut resolver,
                            cancellation,
                            #[cfg(feature = "storage-measurements")]
                            measurement,
                        } => {
                            #[cfg(feature = "storage-measurements")]
                            let started = measurement.as_ref().map(|_| std::time::Instant::now());
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
                            imports.release_if_hidden();
                            #[cfg(feature = "storage-measurements")]
                            let failed = matches!(&output, StorageResult::Failed(_));
                            #[cfg(feature = "storage-measurements")]
                            let executed = measurement.as_ref().map(|_| std::time::Instant::now());
                            resolver.resolve(output);
                            #[cfg(feature = "storage-measurements")]
                            if let Some(measurement) = measurement {
                                measurement.finish(
                                    started.unwrap(),
                                    executed.unwrap(),
                                    std::time::Instant::now(),
                                    failed,
                                );
                            }
                        }
                        Job::Foreground(foreground) => {
                            imports.set_foreground(foreground);
                        }
                    }
                }
                // Closing the bridge drops the sender. Queued operations finish;
                // the connection and worker then exit without blocking Android UI.
            })?;
        Ok(Self {
            sender: Arc::new(sender),
            file,
            #[cfg(feature = "storage-measurements")]
            measurements: None,
        })
    }
}

impl EffectMiddleware for StorageEffects {
    type Op = StorageOperation;

    fn try_process_effect(&self, operation: Self::Op, resolver: EffectResolver<StorageResult>) {
        // Reader/import commands share one app storage slot; the native Anki
        // startup read owns a separate slot on this same FIFO. Publish fresh
        // import attempts only for accepted commands, never for startup reads.
        let cancellation = matches!(
            operation,
            StorageOperation::ImportTxt(_)
                | StorageOperation::ImportSrt(_)
                | StorageOperation::ImportEpub(_)
        )
        .then(|| self.file.begin());
        // The worker exits only when all senders are dropped. No detached task
        // or per-operation thread can reorder database commands.
        #[cfg(feature = "storage-measurements")]
        let measurement = crate::storage_measurements::QueuedMeasurement::new(
            self.measurements.as_ref(),
            &operation,
        );
        assert!(
            self.sender
                .send(Job::Storage {
                    operation,
                    resolver,
                    cancellation,
                    #[cfg(feature = "storage-measurements")]
                    measurement,
                })
                .is_ok(),
            "storage worker exited unexpectedly"
        );
    }
}

fn execute(
    path: &PathBuf,
    storage: &mut Option<Storage>,
    imports: &mut Imports,
    word_loader: &mut WordDictionaryLoader,
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
        StorageOperation::LookupDictionary(query) => {
            let result = word_dictionary(dictionary, word_loader)?
                .lookup(&query.surface, query.dictionary_form.as_deref());
            result.map(StorageResult::DictionaryLoaded).map_err(|_| {
                *dictionary = None;
                DocumentError::Unavailable
            })
        }
        StorageOperation::LoadVocabulary { query, offset } => storage
            .vocabulary_page(&query, offset)
            .map(StorageResult::VocabularyLoaded)
            .map_err(map_error),
        StorageOperation::LoadVocabularyWord(id) => {
            let row = storage
                .vocabulary_row(id)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?;
            let (base, components) = imports.vocabulary_forms(&row.surface)?;
            let dictionary_ref = word_dictionary(dictionary, word_loader)?;
            let (parts, candidates) = dictionary_ref
                .parts(&row.surface, components)
                .and_then(|parts| {
                    dictionary_ref
                        .lookup(&row.surface, base.as_deref())
                        .map(|candidates| (parts, candidates))
                })
                .map_err(|_| {
                    *dictionary = None;
                    DocumentError::Unavailable
                })?;
            Ok(StorageResult::VocabularyWordLoaded(Box::new(
                yomoku_core::vocabulary::VocabularyWordDetails {
                    row,
                    candidates,
                    parts,
                },
            )))
        }
        StorageOperation::SetVocabularyWordStatus(command) => {
            storage
                .set_vocabulary_status(command.id, command.status, command.occurred_at_unix_ms)
                .map_err(map_error)?
                .ok_or(DocumentError::NotFound)?;
            Ok(StorageResult::VocabularyWordStatusChanged(command))
        }
        StorageOperation::LoadStatistics(query) => storage
            .statistics_buckets(query.timeframe, query.as_of_unix_ms)
            .map(|data| StorageResult::StatisticsLoaded(Box::new(data)))
            .map_err(map_error),
        StorageOperation::Effective(operation) => {
            effective::execute(storage, imports, dictionary, word_loader, operation)
                .map(StorageResult::Effective)
        }
        StorageOperation::List => Ok(StorageResult::Listed(
            storage.list_documents().map_err(map_error)?,
            storage.page_length().map_err(map_error)?,
            storage.reader_typography().map_err(map_error)?,
        )),

        StorageOperation::LoadEffectiveLibraryWordDistributions => {
            Ok(StorageResult::LibraryWordDistributionsLoaded(
                storage
                    .effective_library_word_distributions()
                    .map_err(map_error)?,
            ))
        }
        StorageOperation::DeleteDocument(id) => {
            storage.delete_document(id).map_err(map_error)?;
            Ok(StorageResult::DocumentDeleted(id))
        }
        StorageOperation::RenameDocument { document_id, title } => {
            if !storage
                .rename_document(document_id, &title)
                .map_err(map_error)?
            {
                return Err(DocumentError::NotFound);
            }
            Ok(StorageResult::DocumentRenamed { document_id, title })
        }
        StorageOperation::CreatePaste(_) => {
            imports.import(storage, &paste.expect("validated paste"))
        }
        StorageOperation::ImportTxt(input) => imports.import_txt(
            storage,
            &input,
            cancellation.expect("TXT attempt published before enqueue"),
        ),
        StorageOperation::ImportSrt(input) => imports.import_srt(
            storage,
            &input,
            cancellation.expect("SRT attempt published before enqueue"),
        ),
        StorageOperation::ImportEpub(input) => imports.import_epub(
            storage,
            &input,
            cancellation.expect("EPUB attempt published before enqueue"),
        ),

        StorageOperation::OpenReader(id) => storage
            .open_document_reader_snapshot(id)
            .map_err(map_error)?
            .map(StorageResult::Loaded)
            .ok_or(DocumentError::NotFound),

        StorageOperation::LoadStatisticsTimeframe => storage
            .statistics_timeframe()
            .map(StorageResult::StatisticsTimeframeLoaded)
            .map_err(map_error),
        StorageOperation::SaveStatisticsTimeframe(timeframe) => storage
            .save_statistics_timeframe(timeframe)
            .map(StorageResult::StatisticsTimeframeSaved)
            .map_err(map_error),
        StorageOperation::LoadAnkiSettings => storage
            .anki_default_deck()
            .map(StorageResult::AnkiSettingsLoaded)
            .map_err(map_error),
        StorageOperation::SaveAnkiDefaultDeck(deck) => storage
            .save_anki_default_deck(deck)
            .map(StorageResult::AnkiDefaultDeckSaved)
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
    }
}

pub(crate) fn map_error(error: StorageError) -> DocumentError {
    match error {
        StorageError::Sqlite(_) => DocumentError::Unavailable,
        StorageError::InvalidDocument | StorageError::InvalidCompletion => {
            DocumentError::InvalidDocument
        }
        StorageError::InvalidSettings => DocumentError::InvalidSettings,
        StorageError::UnsupportedSchemaVersion { .. }
        | StorageError::InvalidMigrationHistory
        | StorageError::InvalidVocabulary
        | StorageError::InvalidStatistics
        | StorageError::InvalidAnalysis
        | StorageError::UnrecognizedDatabase => DocumentError::IncompatibleDatabase,
    }
}

fn word_dictionary<'a>(
    dictionary: &'a mut Option<Dictionary>,
    loader: &mut WordDictionaryLoader,
) -> Result<&'a Dictionary, DocumentError> {
    if dictionary.is_none() {
        let path = loader()?;
        *dictionary = Some(Dictionary::open(path).map_err(|_| DocumentError::Unavailable)?);
    }
    Ok(dictionary.as_ref().expect("opened dictionary"))
}

fn map_word_error(error: ReaderWordError, dictionary: &mut Option<Dictionary>) -> DocumentError {
    match error {
        ReaderWordError::Storage(error) => map_error(error),
        ReaderWordError::InvalidSelection => DocumentError::InvalidDocument,
        ReaderWordError::Dictionary(_) => {
            // Reopen an atomically repaired index only on explicit retry.
            *dictionary = None;
            DocumentError::Unavailable
        }
    }
}
