//! Narrow native interface. Crux owns the payload format and application state.

use crate::{FfiError, YomokuBridge};
use std::{fs::File, os::fd::FromRawFd, sync::Arc};
use yomoku_analysis::Analyzer;
use yomoku_core::{document::ChapterText, storage::DocumentError};
use yomoku_runtime::{AnalyzerLifecycle, PasteAnalyzer};

/// Trusted platform descriptor publisher, called only on the storage worker.
/// Return a newly owned read-only FD plus one for the verified private Full
/// dictionary, or zero on failure (also the generated callback failure default).
/// Native code consumes/closes it even when loading fails.
/// The published inode MUST remain immutable through analyzer drop, including
/// accepted work draining after bridge close;
/// repairs replace names atomically and never rewrite/truncate published inodes.
/// Never throw across the callback boundary or return a borrowed descriptor.
#[boltffi::export]
pub trait DictionaryProvider: Send + Sync {
    fn open_verified_dictionary(&self) -> i64;
    /// Verify the pinned word/name index size/hash before returning its private,
    /// immutable path. Called only on the FIFO worker, never during construction.
    /// Empty string means failure. Repairs must atomically replace immutable inodes.
    fn verified_word_dictionary_path(&self) -> String;
}

/// Asynchronous storage effects arrive on a Rust worker thread. The host must
/// dispatch UI work to its UI thread and keep callback methods nonblocking.
#[boltffi::export]
pub trait EffectsListener: Send + Sync {
    fn on_effects(&self, requests: Vec<u8>);
    fn on_error(&self, error: FfiError);
}

/// Each native handle owns an independent application instance.
pub struct NativeBridge {
    bridge: YomokuBridge,
    lifecycle: AnalyzerLifecycle,
}

#[boltffi::export]
impl NativeBridge {
    pub fn new(
        database_path: String,
        listener: Box<dyn EffectsListener>,
        dictionary: Box<dyn DictionaryProvider>,
    ) -> Result<Self, FfiError> {
        let dictionary: Arc<dyn DictionaryProvider> = Arc::from(dictionary);
        let words = dictionary.clone();
        let word_loader = Box::new(move || {
            let path = words.verified_word_dictionary_path();
            if path.is_empty() {
                return Err(DocumentError::Unavailable);
            }
            Ok(path.into())
        });
        let loader = Box::new(move || {
            let fd = dictionary
                .open_verified_dictionary()
                .checked_sub(1)
                .and_then(|fd| i32::try_from(fd).ok())
                .filter(|fd| *fd >= 0)
                .ok_or(DocumentError::Unavailable)?;
            // SAFETY: the trusted publisher transfers a newly owned descriptor.
            // No path reopen can race atomic replacement of the published name.
            let file = unsafe { File::from_raw_fd(fd) };
            // SAFETY: DictionaryProvider's private-publication contract forbids
            // writes/truncation of live inodes. The adapter rehashes this mapping.
            let analyzer = unsafe { Analyzer::open_full_mapped(&file) }
                .map_err(|_| DocumentError::Unavailable)?;
            Ok(Box::new(move |chapter: &ChapterText| {
                analyzer
                    .analyze(chapter)
                    .map_err(|_| DocumentError::InvalidDocument)
            }) as Box<dyn PasteAnalyzer>)
        });
        let (bridge, lifecycle) = YomokuBridge::with_dictionaries(
            database_path.into(),
            loader,
            word_loader,
            move |result| match result {
                Ok(requests) => listener.on_effects(requests),
                Err(error) => listener.on_error(error),
            },
        )?;
        Ok(Self { bridge, lifecycle })
    }

    pub fn set_foreground(&self, foreground: bool) -> Result<(), FfiError> {
        self.lifecycle
            .set_foreground(foreground)
            .map_err(|_| FfiError::WorkerUnavailable)
    }

    /// Compatibility entry point to cancel the current file attempt outside FIFO.
    /// Call off Android's main
    /// thread and wait for both this return and its async result before starting
    /// another file. Already committed files remain completed.
    pub fn cancel_txt_import(&self) -> Result<(), FfiError> {
        self.lifecycle
            .cancel_txt_import()
            .map_err(|_| FfiError::WorkerUnavailable)
    }

    /// Cancel the current EPUB/TXT/SRT attempt outside FIFO. As with cancel_txt_import,
    /// call off main and await both this return and the asynchronous outcome.
    pub fn cancel_file_import(&self) -> Result<(), FfiError> {
        self.lifecycle
            .cancel_file_import()
            .map_err(|_| FfiError::WorkerUnavailable)
    }

    pub fn update(&self, event: &[u8]) -> Result<Vec<u8>, FfiError> {
        self.bridge.update(event)
    }

    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<Vec<u8>, FfiError> {
        self.bridge.resolve(id, output)
    }

    pub fn view(&self) -> Result<Vec<u8>, FfiError> {
        self.bridge.view()
    }
}
