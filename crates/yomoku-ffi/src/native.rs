//! Narrow native interface. Crux owns the payload format and application state.

use crate::{FfiError, YomokuBridge};

/// Future asynchronous effects arrive on a Rust worker thread. The host must
/// dispatch UI work to its UI thread and keep callback methods nonblocking.
#[boltffi::export]
pub trait EffectsListener: Send + Sync {
    fn on_effects(&self, requests: Vec<u8>);
    fn on_error(&self, error: FfiError);
}

/// Each native handle owns an independent application instance.
pub struct NativeBridge {
    bridge: YomokuBridge,
}

#[boltffi::export]
impl NativeBridge {
    pub fn new(listener: Box<dyn EffectsListener>) -> Self {
        Self {
            bridge: YomokuBridge::new(move |result| match result {
                Ok(requests) => listener.on_effects(requests),
                Err(error) => listener.on_error(error),
            }),
        }
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
