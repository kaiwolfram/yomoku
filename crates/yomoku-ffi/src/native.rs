//! Narrow native interface. Crux owns the payload format and application state.

use crate::{FfiError, YomokuBridge};
use std::{fs::File, os::fd::FromRawFd, sync::Arc};
use yomoku_analysis::{Analyzer, DICTIONARY_REVISION, ENGINE_REVISION};
use yomoku_core::{
    analysis::{AnalysisProfile, ChapterAnalysis, SplitMode},
    document::{ChapterText, DocumentId},
    reader_images::ReaderImageAsset,
    storage::DocumentError,
};
use yomoku_runtime::{AnalyzerLifecycle, PasteAnalyzer};

struct NativeAnalyzer(Analyzer);

impl PasteAnalyzer for NativeAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        self.0
            .analyze(chapter)
            .map_err(|_| DocumentError::InvalidDocument)
    }

    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        if profile.engine_revision != ENGINE_REVISION
            || profile.dictionary_revision != DICTIONARY_REVISION
        {
            return Err(DocumentError::Unavailable);
        }
        self.0
            .analyze_mode(chapter, mode)
            .map_err(|_| DocumentError::InvalidDocument)
    }
}

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

#[boltffi::error]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpubAssetError {
    InvalidRequest,
    NotFound,
    InvalidDocument,
    Unavailable,
}

/// One-shot asset response on the Rust FIFO worker. Keep callbacks nonblocking;
/// decode images on a separate platform worker and publish only to a still-current
/// document/page. Accepted callbacks can arrive after NativeBridge.close().
#[boltffi::export]
pub trait EpubAssetListener: Send + Sync {
    fn on_asset(&self, media_type: String, bytes: Vec<u8>);
    fn on_error(&self, error: EpubAssetError);
}

/// One-shot metadata response on the FIFO worker, including after bridge close.
/// The 32-byte digest identifies cached pixels; verify bytes with the asset API
/// before decoding a new cache entry. No cover also covers absent/non-EPUB IDs.
#[boltffi::export]
pub trait EpubCoverListener: Send + Sync {
    fn on_cover(&self, path: String, media_type: String, sha256: Vec<u8>);
    fn on_no_cover(&self);
    fn on_error(&self, error: EpubAssetError);
}

/// Each native handle owns an independent application instance.
pub struct NativeBridge {
    bridge: YomokuBridge,
    lifecycle: AnalyzerLifecycle,
}

#[boltffi::export]
impl NativeBridge {
    /// Fetch a cover reference without loading book content or image bytes.
    pub fn load_epub_cover_reference(
        &self,
        document_id: i64,
        listener: Box<dyn EpubCoverListener>,
    ) -> Result<(), EpubAssetError> {
        let document_id =
            DocumentId::try_from(document_id).map_err(|_| EpubAssetError::InvalidRequest)?;
        self.lifecycle
            .load_epub_cover_reference(document_id, move |result| match result {
                Ok(Some(cover)) => listener.on_cover(
                    cover.asset.path,
                    cover.asset.media_type,
                    cover.sha256.to_vec(),
                ),
                Ok(None) => listener.on_no_cover(),
                Err(error) => listener.on_error(match error {
                    DocumentError::InvalidDocument => EpubAssetError::InvalidDocument,
                    _ => EpubAssetError::Unavailable,
                }),
            })
            .map_err(|_| EpubAssetError::Unavailable)
    }

    /// Enqueue one verified image read. This never loads an analyzer or puts
    /// image bytes into Crux state. Invalid identities fail before enqueue; all
    /// other outcomes use the per-request listener, retained only until delivery.
    pub fn load_epub_image_asset(
        &self,
        document_id: i64,
        path: String,
        media_type: String,
        listener: Box<dyn EpubAssetListener>,
    ) -> Result<(), EpubAssetError> {
        let document_id =
            DocumentId::try_from(document_id).map_err(|_| EpubAssetError::InvalidRequest)?;
        let asset = ReaderImageAsset { path, media_type };
        asset
            .validate()
            .map_err(|_| EpubAssetError::InvalidRequest)?;
        self.lifecycle
            .load_epub_image_asset(document_id, asset, move |result| match result {
                Ok(image) => listener.on_asset(image.media_type, image.bytes),
                Err(error) => listener.on_error(match error {
                    DocumentError::NotFound => EpubAssetError::NotFound,
                    DocumentError::InvalidDocument => EpubAssetError::InvalidDocument,
                    _ => EpubAssetError::Unavailable,
                }),
            })
            .map_err(|_| EpubAssetError::Unavailable)
    }

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
            Ok(Box::new(NativeAnalyzer(analyzer)) as Box<dyn PasteAnalyzer>)
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
