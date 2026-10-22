//! Serialized host boundary; native Kotlin exports are a subsequent slice.
//!
//! Payloads use Crux's Bincode format and typed event/request/view schemas.
//! Each instance owns an independent core. No global singleton or platform I/O.

use crux_core::{
    bridge::{BincodeFfiFormat, BridgeError, EffectId},
    middleware::{Bridge, Layer},
};
use yomoku_runtime::{effect_router, EffectRouter};

/// Stable error categories for the future native binding; payloads and internal
/// error strings are intentionally not included.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FfiError {
    InvalidEvent,
    InvalidEffectOutput,
    InvalidResponse,
    RequestEncoding,
    ViewEncoding,
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
    /// The callback delivers effects produced after an asynchronous Rust
    /// operation completes. Hosts must dispatch it onto their UI thread.
    pub fn new(on_effects: impl Fn(Result<Vec<u8>, FfiError>) + Send + Sync + 'static) -> Self {
        Self {
            bridge: effect_router().bridge(move |result| on_effects(result.map_err(Into::into))),
        }
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
