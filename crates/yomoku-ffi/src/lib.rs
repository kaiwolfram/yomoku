//! Serialized host boundary and native exports.
//!
//! Payloads use Crux's Bincode format and typed event/request/view schemas.
//! Each instance owns an independent core. No global singleton or platform I/O.

use crux_core::{
    bridge::{BincodeFfiFormat, BridgeError, EffectId},
    middleware::{Bridge, Layer},
};
use yomoku_runtime::{
    effect_router, AnalyzerLifecycle, AnalyzerLoader, EffectRouter, WordDictionaryLoader,
};

pub mod native;

/// Error categories for the native binding; payloads and internal
/// error strings are intentionally not included.
#[boltffi::error]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FfiError {
    InvalidEvent,
    InvalidEffectOutput,
    InvalidResponse,
    RequestEncoding,
    ViewEncoding,
    WorkerUnavailable,
}

impl From<BridgeError<BincodeFfiFormat>> for FfiError {
    fn from(error: BridgeError<BincodeFfiFormat>) -> Self {
        match error {
            BridgeError::DeserializeEvent(_) => Self::InvalidEvent,
            BridgeError::DeserializeOutput(_) => Self::InvalidEffectOutput,
            BridgeError::ProcessResponse(_) => Self::InvalidResponse,
            BridgeError::SerializeRequests(_) => Self::RequestEncoding,
            BridgeError::SerializeView(_) => Self::ViewEncoding,
        }
    }
}

pub struct YomokuBridge {
    bridge: Bridge<EffectRouter, BincodeFfiFormat>,
}

impl YomokuBridge {
    /// Host-only diagnostic factory; the production Android API is unchanged.
    #[cfg(feature = "storage-measurements")]
    pub fn with_measured_effective_reader(
        database_path: std::path::PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
        observer: yomoku_runtime::storage_measurements::StorageObserver,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<(Self, AnalyzerLifecycle), FfiError> {
        let (router, lifecycle) = yomoku_runtime::effect_router_with_measured_effective_reader(
            database_path,
            loader,
            word_loader,
            observer,
        )
        .map_err(|_| FfiError::WorkerUnavailable)?;
        Ok((
            Self {
                bridge: router.bridge(move |result| on_effects(result.map_err(Into::into))),
            },
            lifecycle,
        ))
    }

    /// Production effective reader with retained native provenance, sharing the
    /// import/dictionary worker and lifecycle without a shell toggle.
    pub fn new(
        database_path: std::path::PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<(Self, AnalyzerLifecycle), FfiError> {
        let (router, lifecycle) = effect_router(database_path, loader, word_loader)
            .map_err(|_| FfiError::WorkerUnavailable)?;
        Ok((
            Self {
                bridge: router.bridge(move |result| on_effects(result.map_err(Into::into))),
            },
            lifecycle,
        ))
    }

    pub fn update(&self, event: &[u8]) -> Result<Vec<u8>, FfiError> {
        let mut requests = Vec::new();
        self.bridge.update(event, &mut requests)?;
        Ok(requests)
    }

    /// Render is fire-and-forget and must not be resolved. Unknown, completed,
    /// and non-resolvable request IDs return InvalidResponse.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<Vec<u8>, FfiError> {
        let mut requests = Vec::new();
        self.bridge.resolve(EffectId(id), output, &mut requests)?;
        Ok(requests)
    }

    pub fn view(&self) -> Result<Vec<u8>, FfiError> {
        let mut view = Vec::new();
        self.bridge.view(&mut view)?;
        Ok(view)
    }
}
