use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Yomoku},
    document::LibraryDocument,
    storage::{DocumentError, StorageResult},
};

fn documents() -> Vec<LibraryDocument> {
    [
        "猫の本 Rust",
        "RUST 入門",
        "題𠮷か\u{3099}\0%_",
        "Other title",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, title)| LibraryDocument {
        last_read_sequence: None,
        document_id: (index as i64 + 1).try_into().unwrap(),
        title: title.into(),
        progress_percent: index as u64 * 20,
        finished_at_least_once: index == 2,
    })
    .collect()
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

fn load(core: &Core<Yomoku>) {
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        core,
        &mut effects,
        StorageResult::Listed(documents(), Default::default(), Default::default()),
    );
}

#[test]
fn search_projects_literal_unicode_titles_without_changing_rows_or_order() {
    let core = Core::<Yomoku>::new();
    load(&core);
    let original = core.view();
    for (query, expected) in [
        ("rust", vec![1, 0]),
        ("猫", vec![0]),
        ("𠮷か\u{3099}\0%_", vec![2]),
        ("%_", vec![2]),
        ("  ", vec![]),
        ("no matching title", vec![]),
        ("", vec![3, 2, 1, 0]),
    ] {
        let effects = core.process_event(Event::SearchLibraryTitles(query.into()));
        assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
        let view = core.view();
        assert_eq!(view.library_title_query, query);
        assert_eq!(view.library_document_count, 4);
        assert_eq!(
            view.documents,
            expected
                .into_iter()
                .map(|index| documents()[index].clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(view.document_error, original.document_error);
        assert_eq!(view.reader, original.reader);
    }
    assert_eq!(core.view(), original);
}

#[test]
fn search_during_refresh_retains_full_library_and_survives_failures_and_navigation() {
    let core = Core::<Yomoku>::new();
    load(&core);
    let mut effects = core.process_event(Event::RefreshLibrary);
    let search = core.process_event(Event::SearchLibraryTitles("RUST".into()));
    assert!(matches!(search.as_slice(), [Effect::Render(_)]));
    assert!(core.view().storage_busy);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::SearchLibraryTitles("猫".into()));
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    load(&core);
    assert_eq!(core.view().documents, vec![documents()[0].clone()]);
    for event in [
        Event::OpenSettings,
        Event::CloseSettings,
        Event::SelectDestination(Destination::Vocabulary),
        Event::SelectDestination(Destination::Library),
        Event::OpenPaste,
        Event::CloseDocument,
    ] {
        core.process_event(event);
        assert_eq!(core.view().library_title_query, "猫");
    }
    core.process_event(Event::SearchLibraryTitles(String::new()));
    assert_eq!(
        core.view().documents,
        documents().into_iter().rev().collect::<Vec<_>>()
    );
    assert_eq!(Core::<Yomoku>::new().view().library_title_query, "");
}

#[test]
fn hidden_search_callbacks_cannot_replace_the_retained_query() {
    let core = Core::<Yomoku>::new();
    load(&core);
    core.process_event(Event::SearchLibraryTitles("猫".into()));
    for event in [
        Event::OpenSettings,
        Event::SelectDestination(Destination::Statistics),
        Event::OpenPaste,
        Event::OpenReader(1.try_into().unwrap()),
    ] {
        core.process_event(event);
        assert!(core
            .process_event(Event::SearchLibraryTitles("late callback".into()))
            .is_empty());
        assert_eq!(core.view().library_title_query, "猫");
    }
}

#[test]
fn import_and_duplicate_results_obey_search_without_hiding_canonical_updates() {
    let core = Core::<Yomoku>::new();
    load(&core);
    core.process_event(Event::SearchLibraryTitles("unmatched".into()));
    let mut created = documents()[0].clone();
    created.document_id = 5.try_into().unwrap();
    let mut effects = core.process_event(Event::CreatePaste("input".into()));
    finish(&core, &mut effects, StorageResult::Created(created.clone()));
    assert!(core.view().documents.is_empty());
    assert_eq!(core.view().library_document_count, 5);
    assert_eq!(
        core.view().import_summary.as_ref().unwrap().document_id,
        created.document_id
    );
    let mut existing = documents()[2].clone();
    existing.progress_percent = 91;
    let mut effects = core.process_event(Event::CreatePaste("same input".into()));
    finish(
        &core,
        &mut effects,
        StorageResult::Existing(existing.clone()),
    );
    core.process_event(Event::SearchLibraryTitles(String::new()));
    let view = core.view();
    assert_eq!(view.library_document_count, 5);
    assert_eq!(view.documents[2], existing);
    assert_eq!(view.documents[0], created);
}
