use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Requests};
use yomoku_core::app::{Destination, Event, ViewModel};
use yomoku_ffi::{FfiError, YomokuBridge};
use yomoku_runtime::ShellEffectFfi;

fn bridge() -> YomokuBridge {
    YomokuBridge::new(":memory:".into(), |_| {
        panic!("navigation must produce synchronous effects")
    })
    .unwrap()
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

#[test]
fn document_values_reject_invalid_numbers_when_decoding_ffi_payloads() {
    use yomoku_core::active_time::PendingReadingTime;
    use yomoku_core::document::{ChapterId, DocumentId, SourceId, TextOffset};

    for value in [i64::MIN, -1, 0, 1, (1 << 53) + 1, i64::MAX] {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &value).unwrap();
        assert_eq!(
            BincodeFfiFormat::deserialize::<PendingReadingTime>(&bytes).is_ok(),
            value >= 0
        );
        assert_eq!(
            BincodeFfiFormat::deserialize::<DocumentId>(&bytes).is_ok(),
            value > 0
        );
        assert_eq!(
            BincodeFfiFormat::deserialize::<SourceId>(&bytes).is_ok(),
            value > 0
        );
        assert_eq!(
            BincodeFfiFormat::deserialize::<ChapterId>(&bytes).is_ok(),
            value > 0
        );
        assert_eq!(
            BincodeFfiFormat::deserialize::<TextOffset>(&bytes).is_ok(),
            value >= 0
        );
    }
}
