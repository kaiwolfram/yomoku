use std::sync::{Arc, Weak};

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::app::{Destination, Event, ViewModel};
use yomoku_ffi::{
    native::{EffectsListener, NativeBridge},
    FfiError,
};
use yomoku_runtime::ShellEffectFfi;

struct Listener {
    _lifetime: Arc<()>,
}

impl EffectsListener for Listener {
    fn on_effects(&self, _: Vec<u8>) {
        panic!("navigation must return effects synchronously");
    }

    fn on_error(&self, _: FfiError) {
        panic!("navigation must return errors synchronously");
    }
}

fn bridge() -> (NativeBridge, Weak<()>) {
    let lifetime = Arc::new(());
    let weak = Arc::downgrade(&lifetime);
    (
        NativeBridge::new(Box::new(Listener {
            _lifetime: lifetime,
        })),
        weak,
    )
}

fn view(bridge: &NativeBridge) -> ViewModel {
    BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap()
}

#[test]
fn native_facade_preserves_payloads_errors_and_instance_isolation() {
    let (first, _) = bridge();
    let (second, _) = bridge();
    let mut event = Vec::new();
    BincodeFfiFormat::serialize(
        &mut event,
        &Event::SelectDestination(Destination::Vocabulary),
    )
    .unwrap();
    let requests: Requests<ShellEffectFfi> =
        BincodeFfiFormat::deserialize(&first.update(&event).unwrap()).unwrap();
    assert_eq!(requests.0.len(), 1);
    assert!(matches!(requests.0[0].effect, ShellEffectFfi::Render(_)));
    assert_eq!(first.update(&[]), Err(FfiError::InvalidEvent));
    assert_eq!(
        first.resolve(requests.0[0].id.0, &[]),
        Err(FfiError::InvalidResponse)
    );
    assert_eq!(view(&first).destination, Destination::Vocabulary);
    assert_eq!(view(&second).destination, Destination::Library);
}

#[test]
fn native_facade_can_be_shared_across_threads_and_releases_listener() {
    let (bridge, lifetime) = bridge();
    let bridge = Arc::new(bridge);
    let worker = Arc::clone(&bridge);
    std::thread::spawn(move || {
        assert_eq!(view(&worker).destination, Destination::Library);
    })
    .join()
    .unwrap();
    assert!(lifetime.upgrade().is_some());
    drop(bridge);
    assert!(lifetime.upgrade().is_none());
}
