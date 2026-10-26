//! Image payloads bypass pure reader state but share its single SQLite FIFO.

use yomoku_core::{document::DocumentId, reader_images::ReaderImageAsset, storage::DocumentError};
use yomoku_import::epub::EpubImage;

use crate::{imports::worker_closed, storage::Job, AnalyzerLifecycle};

pub(crate) type AssetCallback = Box<dyn FnOnce(Result<EpubImage, DocumentError>) + Send + 'static>;

impl AnalyzerLifecycle {
    /// Enqueue a bounded read without changing core state or loading dictionaries.
    /// The callback runs once on the existing storage worker; keep it nonblocking.
    /// Accepted reads drain after router closure. This weak control handle alone
    /// does not retain the worker; submission after closure fails synchronously.
    pub fn load_epub_image_asset(
        &self,
        document_id: DocumentId,
        asset: ReaderImageAsset,
        callback: impl FnOnce(Result<EpubImage, DocumentError>) + Send + 'static,
    ) -> std::io::Result<()> {
        self.sender
            .upgrade()
            .ok_or_else(worker_closed)?
            .send(Job::EpubAsset(document_id, asset, Box::new(callback)))
            .map_err(|_| worker_closed())
    }
}
