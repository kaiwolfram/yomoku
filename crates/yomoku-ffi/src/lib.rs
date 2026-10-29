//! Serialized host boundary and native exports.
//!
//! Payloads use Crux's Bincode format and typed event/request/view schemas.
//! Each instance owns an independent core. No global singleton or platform I/O.

use crux_core::{
    bridge::{BincodeFfiFormat, BridgeError, EffectId},
    middleware::{Bridge, Layer},
};
use yomoku_runtime::{
    effect_router, effect_router_with_analyzed_imports, effect_router_with_dictionaries,
    effect_router_with_effective_reader, AnalyzerLifecycle, AnalyzerLoader, EffectRouter,
    WordDictionaryLoader,
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
    /// Serialized effective-reader activation candidate. Uses the same native
    /// worker/lifecycle as production; no shell toggle or extra wire protocol.
    /// NativeBridge remains on original B until coordinated activation.
    pub fn with_effective_reader(
        database_path: std::path::PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<(Self, AnalyzerLifecycle), FfiError> {
        let (router, lifecycle) =
            effect_router_with_effective_reader(database_path, loader, word_loader)
                .map_err(|_| FfiError::WorkerUnavailable)?;
        Ok((
            Self {
                bridge: router.bridge(move |result| on_effects(result.map_err(Into::into))),
            },
            lifecycle,
        ))
    }

    pub fn with_dictionaries(
        database_path: std::path::PathBuf,
        loader: AnalyzerLoader,
        word_loader: WordDictionaryLoader,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<(Self, AnalyzerLifecycle), FfiError> {
        let (router, lifecycle) =
            effect_router_with_dictionaries(database_path, loader, word_loader)
                .map_err(|_| FfiError::WorkerUnavailable)?;
        Ok((
            Self {
                bridge: router.bridge(move |result| on_effects(result.map_err(Into::into))),
            },
            lifecycle,
        ))
    }

    /// Production imports require a loader; no text-only fallback is selected.
    pub fn with_analyzed_imports(
        database_path: std::path::PathBuf,
        loader: AnalyzerLoader,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<(Self, AnalyzerLifecycle), FfiError> {
        let (router, lifecycle) = effect_router_with_analyzed_imports(database_path, loader)
            .map_err(|_| FfiError::WorkerUnavailable)?;
        Ok((
            Self {
                bridge: router.bridge(move |result| on_effects(result.map_err(Into::into))),
            },
            lifecycle,
        ))
    }

    /// The callback delivers effects produced after an asynchronous Rust
    /// operation completes. Hosts must dispatch it onto their UI thread.
    pub fn new(
        database_path: std::path::PathBuf,
        on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static,
    ) -> Result<Self, FfiError> {
        Ok(Self {
            bridge: effect_router(database_path)
                .map_err(|_| FfiError::WorkerUnavailable)?
                .bridge(move |result| on_effects(result.map_err(Into::into))),
        })
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
