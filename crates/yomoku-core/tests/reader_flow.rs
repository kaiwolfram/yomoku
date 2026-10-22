use crux_core::Core;
use yomoku_core::{
    app::{ContentRoute, Destination, Effect, Event, Yomoku},
    document::{ChapterId, ChapterText, DocumentId, PastedDocument, SourceId},
    storage::{DocumentError, StorageOperation, StorageResult},
};

fn document(text: &str) -> PastedDocument {
    PastedDocument {
        document_id: DocumentId::try_from(1).unwrap(),
        title: "題𠮷".into(),
        chapter: ChapterText {
            source_id: SourceId::try_from(2).unwrap(),
            chapter_id: ChapterId::try_from(3).unwrap(),
            text: text.into(),
        },
    }
}

fn finish(core: &Core<Yomoku>, effects: &mut [Effect], result: StorageResult) {
    let request = effects
        .iter_mut()
        .find_map(|effect| match effect {
            Effect::Storage(request) => Some(request),
            _ => None,
        })
        .unwrap();
    core.resolve(request, result).unwrap();
}

#[test]
fn paste_success_returns_to_library_without_opening_reader_and_failures_keep_form() {
    let core = Core::<Yomoku>::new();
    core.process_event(Event::SelectDestination(Destination::Nostr));
    core.process_event(Event::OpenPaste);
    assert_eq!(core.view().destination, Destination::Library);
    assert_eq!(core.view().content_route, ContentRoute::Paste);
    let mut effects = core.process_event(Event::CreatePaste(" \n".into()));
    assert!(core
        .process_event(Event::CreatePaste("duplicate".into()))
        .is_empty());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::EmptyPaste),
    );
    assert_eq!(core.view().content_route, ContentRoute::Paste);
    assert_eq!(core.view().document_error, Some(DocumentError::EmptyPaste));
    let mut effects = core.process_event(Event::CreatePaste("題𠮷".into()));
    assert!(core.view().document_error.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::Created(document("題𠮷")),
    );
    let view = core.view();
    assert_eq!(view.content_route, ContentRoute::Main);
    assert_eq!(view.documents.len(), 1);
    assert_eq!(view.documents[0].title, "題𠮷");
    assert!(view.reader.is_none());
    assert!(view.loaded_document.is_none());
}

#[test]
fn reader_opens_exact_first_logical_page_with_unicode_anchor_and_counter() {
    let core = Core::<Yomoku>::new();
    let first = format!("{}。\r\n", "𠮷".repeat(499));
    let document = document(&format!("{first}次の段落。"));
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    assert_eq!(core.view().content_route, ContentRoute::Reader);
    assert!(core.view().storage_busy);
    assert!(core.view().reader.is_none());
    assert!(effects.iter().any(|effect| matches!(effect,
        Effect::Storage(request) if request.operation == StorageOperation::LoadPaste(document.document_id))));
    finish(&core, &mut effects, StorageResult::Loaded(document.clone()));
    let page = core.view().reader.unwrap();
    assert_eq!(page.text, first);
    assert_eq!(page.anchor.slice(&document.chapter).unwrap(), first);
    assert_eq!(i64::from(page.position.offset), 0);
    assert_eq!(page.position.chapter_id, document.chapter.chapter_id);
    assert_eq!((page.page_number, page.total_pages), (1, 2));
    core.process_event(Event::OpenSettings);
    core.process_event(Event::CloseSettings);
    assert_eq!(core.view().reader, Some(page));
    core.process_event(Event::CloseDocument);
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader.is_none());
    assert!(Core::<Yomoku>::new().view().reader.is_none());
}

#[test]
fn cancelled_reader_loads_never_reopen_after_back_tab_or_paste_navigation() {
    for cancel in [
        Event::CloseDocument,
        Event::SelectDestination(Destination::Vocabulary),
        Event::OpenPaste,
    ] {
        let core = Core::<Yomoku>::new();
        let document = document("短い本文。\n");
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        core.process_event(cancel);
        let route = core.view().content_route;
        finish(&core, &mut effects, StorageResult::Loaded(document));
        assert_eq!(core.view().content_route, route);
        assert!(core.view().reader.is_none());
        assert!(!core.view().storage_busy);
    }
}

#[test]
fn failed_reader_can_return_to_library_and_retry_while_plain_load_never_opens_it() {
    let core = Core::<Yomoku>::new();
    let document = document("短い本文。\n");
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().content_route, ContentRoute::Reader);
    assert!(core.view().reader.is_none());
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    core.process_event(Event::CloseDocument);
    assert!(core.view().document_error.is_none());
    let mut effects = core.process_event(Event::LoadPaste(document.document_id));
    finish(&core, &mut effects, StorageResult::Loaded(document.clone()));
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader.is_none());
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, StorageResult::Loaded(document));
    assert_eq!(core.view().reader.unwrap().total_pages, 1);
}
