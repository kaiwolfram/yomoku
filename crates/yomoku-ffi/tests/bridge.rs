use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::app::{Destination, Event, ViewModel};
use yomoku_ffi::{FfiError, YomokuBridge};
use yomoku_runtime::ShellEffectFfi;

mod support;
fn bridge() -> YomokuBridge {
    support::bridge(":memory:".into(), |_| {
        panic!("navigation must produce synchronous effects")
    })
    .unwrap()
    .0
}

fn select(bridge: &YomokuBridge, destination: Destination) -> Requests<ShellEffectFfi> {
    let mut event = Vec::new();
    BincodeFfiFormat::serialize(&mut event, &Event::SelectDestination(destination)).unwrap();
    BincodeFfiFormat::deserialize(&bridge.update(&event).unwrap()).unwrap()
}

fn view(bridge: &YomokuBridge) -> ViewModel {
    BincodeFfiFormat::deserialize(&bridge.view().unwrap()).unwrap()
}

#[test]
fn typed_events_requests_and_views_round_trip_through_router() {
    let bridge = bridge();
    assert_eq!(view(&bridge).destination, Destination::Library);
    for destination in [
        Destination::Statistics,
        Destination::Statistics,
        Destination::Vocabulary,
        Destination::Library,
    ] {
        let requests = select(&bridge, destination);
        assert_eq!(requests.0.len(), 1);
        assert!(matches!(requests.0[0].effect, ShellEffectFfi::Render(_)));
        assert_eq!(view(&bridge).destination, destination);
    }
}

#[test]
fn invalid_events_preserve_state_and_bridge_remains_usable() {
    let bridge = bridge();
    select(&bridge, Destination::Vocabulary);
    for bytes in [
        &[][..],
        &[0, 0, 0],
        &[255; 8],
        &[0, 0, 0, 0, 255, 255, 255, 255],
    ] {
        assert_eq!(bridge.update(bytes), Err(FfiError::InvalidEvent));
        assert_eq!(view(&bridge).destination, Destination::Vocabulary);
    }
    select(&bridge, Destination::Library);
    assert_eq!(view(&bridge).destination, Destination::Library);
}

#[test]
fn unknown_and_render_response_ids_return_errors_without_changing_state() {
    let bridge = bridge();
    assert_eq!(
        bridge.resolve(u32::MAX, &[]),
        Err(FfiError::InvalidResponse)
    );
    let requests = select(&bridge, Destination::Statistics);
    for _ in 0..2 {
        assert_eq!(
            bridge.resolve(requests.0[0].id.0, &[]),
            Err(FfiError::InvalidResponse)
        );
    }
    assert_eq!(view(&bridge).destination, Destination::Statistics);
    select(&bridge, Destination::Library);
    assert_eq!(view(&bridge).destination, Destination::Library);
}

#[test]
fn bridge_instances_have_independent_state() {
    let first = bridge();
    select(&first, Destination::Statistics);
    assert_eq!(view(&bridge()).destination, Destination::Library);
    assert_eq!(view(&first).destination, Destination::Statistics);
}
