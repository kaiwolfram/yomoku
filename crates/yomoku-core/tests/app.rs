use crux_core::Core;
use yomoku_core::app::{Destination, Effect, Event, ViewModel, Yomoku};

#[test]
fn starts_in_library() {
    let core = Core::<Yomoku>::new();
    assert_eq!(
        core.view(),
        ViewModel {
            library_word_distributions: None,
            library_word_error: None,
            reader_time: None,
            reading_time_save_pending: false,
            completion_save_pending: false,
            completion_notice: None,
            book_completion_notice: None,
            destination: Destination::Library,
            settings_open: false,
            documents: vec![],
            library_title_query: String::new(),
            library_sort: Default::default(),
            library_document_count: 0,
            loaded_document: None,
            storage_busy: false,
            document_error: None,
            content_route: yomoku_core::app::ContentRoute::Main,
            reader: None,
            reader_analysis: None,
            reader_tokens: None,
            reader_furigana: Vec::new(),
            reader_word_selection: None,
            reader_word: None,
            reader_word_error: None,
            reader_custom_entry_error: None,
            reader_typography: None,
            page_length: None,
            import_summary: None,
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

#[test]
fn settings_returns_to_each_selected_destination_and_is_not_restored() {
    let core = Core::<Yomoku>::new();
    for destination in [
        Destination::Library,
        Destination::Nostr,
        Destination::Statistics,
        Destination::Vocabulary,
    ] {
        let _ = core.process_event(Event::SelectDestination(destination));
        for event in [
            Event::OpenSettings,
            Event::OpenSettings,
            Event::CloseSettings,
            Event::CloseSettings,
        ] {
            let effects = core.process_event(event.clone());
            assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
            assert_eq!(core.view().destination, destination);
            assert_eq!(
                core.view().settings_open,
                matches!(event, Event::OpenSettings)
            );
        }
    }
    let _ = core.process_event(Event::OpenSettings);
    assert!(!Core::<Yomoku>::new().view().settings_open);
    let _ = core.process_event(Event::SelectDestination(Destination::Library));
    assert!(!core.view().settings_open);
}

#[test]
fn storage_commands_are_serialized_while_navigation_remains_available() {
    use yomoku_core::storage::{DocumentError, StorageOperation, StorageResult};
    let core = Core::<Yomoku>::new();
    let mut effects = core.process_event(Event::RefreshLibrary);
    assert!(core.view().storage_busy);
    assert!(core
        .process_event(Event::CreatePaste("duplicate".into()))
        .is_empty());
    let _ = core.process_event(Event::SelectDestination(Destination::Vocabulary));
    assert_eq!(core.view().destination, Destination::Vocabulary);
    let request = effects
        .iter_mut()
        .find_map(|effect| match effect {
            Effect::Storage(request) => Some(request),
            Effect::Render(_) => None,
        })
        .unwrap();
    assert_eq!(request.operation, StorageOperation::List);
    core.resolve(request, StorageResult::Failed(DocumentError::Unavailable))
        .unwrap();
    assert!(!core.view().storage_busy);
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    assert!(!core.process_event(Event::RefreshLibrary).is_empty());
    assert!(core.view().document_error.is_none());
}
