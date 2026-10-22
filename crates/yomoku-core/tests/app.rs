use crux_core::Core;
use yomoku_core::app::{Destination, Effect, Event, ViewModel, Yomoku};

#[test]
fn starts_in_library() {
    let core = Core::<Yomoku>::new();
    assert_eq!(
        core.view(),
        ViewModel {
            destination: Destination::Library,
        }
    );
}

#[test]
fn destination_commands_update_view_and_request_one_render() {
    let core = Core::<Yomoku>::new();
    for destination in [
        Destination::Nostr,
        Destination::Statistics,
        Destination::Vocabulary,
        Destination::Library,
    ] {
        let effects = core.process_event(Event::SelectDestination(destination));
        assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
        assert_eq!(core.view().destination, destination);
    }
}

#[test]
fn reselecting_library_can_request_initial_render() {
    let core = Core::<Yomoku>::new();
    for _ in 0..2 {
        let effects = core.process_event(Event::SelectDestination(Destination::Library));
        assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
        assert_eq!(core.view().destination, Destination::Library);
    }
}

#[test]
fn new_core_does_not_restore_transient_navigation() {
    let core = Core::<Yomoku>::new();
    let _ = core.process_event(Event::SelectDestination(Destination::Vocabulary));
    assert_eq!(
        Core::<Yomoku>::new().view().destination,
        Destination::Library
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
}
