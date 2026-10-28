use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Yomoku},
    document::{DocumentId, LibraryDocument},
    storage::{DocumentError, StorageOperation, StorageResult},
};

fn id(n: i64) -> DocumentId {
    n.try_into().unwrap()
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

fn loaded() -> Core<Yomoku> {
    let core = Core::<Yomoku>::new();
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        &core,
        &mut effects,
        StorageResult::Listed(
            (1..=2)
                .map(|n| LibraryDocument {
                    document_id: id(n),
                    title: format!("本𠮷 {n}"),
                    progress_percent: 75,
                    finished_at_least_once: true,
                    last_read_sequence: Some(n),
                })
                .collect(),
            Default::default(),
            Default::default(),
        ),
    );
    core
}

#[test]
fn document_deletion_requires_visible_frozen_target_and_explicit_matching_confirmation() {
    let core = loaded();
    let before = core.view();
    assert!(core
        .process_event(Event::ConfirmDocumentDeletion(id(1)))
        .is_empty());
    assert!(core
        .process_event(Event::RequestDocumentDeletion(id(999)))
        .is_empty());
    let effects = core.process_event(Event::RequestDocumentDeletion(id(1)));
    assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
    assert_eq!(
        core.view().library_delete_target,
        Some(before.documents[1].clone())
    );
    assert!(core
        .process_event(Event::ConfirmDocumentDeletion(id(2)))
        .is_empty());
    core.process_event(Event::SearchLibraryTitles("2".into()));
    core.process_event(Event::SortLibrary(yomoku_core::library::LibrarySort::Title));
    assert_eq!(
        core.view().library_delete_target,
        Some(before.documents[1].clone())
    );
    core.process_event(Event::CancelDocumentDeletion);
    assert!(core.view().library_delete_target.is_none());
    assert!(core
        .process_event(Event::ConfirmDocumentDeletion(id(1)))
        .is_empty());
    assert!(core
        .process_event(Event::RequestDocumentDeletion(id(1)))
        .is_empty());
    assert_eq!(core.view().library_document_count, 2);
}

#[test]
fn document_deletion_success_updates_full_library_counts_and_rejects_double_submissions() {
    let core = loaded();
    core.process_event(Event::RequestDocumentDeletion(id(1)));
    let mut effects = core.process_event(Event::ConfirmDocumentDeletion(id(1)));
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Storage(request) if request.operation == StorageOperation::DeleteDocument(id(1)))));
    assert!(core.view().storage_busy);
    assert!(core
        .process_event(Event::ConfirmDocumentDeletion(id(1)))
        .is_empty());
    assert!(core
        .process_event(Event::RequestDocumentDeletion(id(2)))
        .is_empty());
    assert!(core.process_event(Event::CancelDocumentDeletion).is_empty());
    core.process_event(Event::SearchLibraryTitles("1".into()));
    finish(&core, &mut effects, StorageResult::DocumentDeleted(id(1)));
    let view = core.view();
    assert!(!view.storage_busy);
    assert!(view.library_delete_target.is_none());
    assert!(view.documents.is_empty());
    assert_eq!(view.library_document_count, 1);
    assert!(view.library_word_distributions.is_none());
    assert_eq!(view.library_title_query, "1");
    core.process_event(Event::SearchLibraryTitles(String::new()));
    assert_eq!(core.view().documents[0].document_id, id(2));
}

#[test]
fn document_deletion_failure_retains_confirmation_for_explicit_retry_and_separate_errors() {
    let core = loaded();
    let mut effects = core.process_event(Event::CreatePaste("".into()));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::EmptyPaste),
    );
    core.process_event(Event::RequestDocumentDeletion(id(1)));
    let target = core.view().library_delete_target;
    let mut effects = core.process_event(Event::ConfirmDocumentDeletion(id(1)));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let view = core.view();
    assert_eq!(view.library_delete_error, Some(DocumentError::Unavailable));
    assert_eq!(view.document_error, Some(DocumentError::EmptyPaste));
    assert_eq!(view.library_delete_target, target);
    assert_eq!(view.library_document_count, 2);
    assert!(!view.storage_busy);
    assert!(core
        .process_event(Event::DismissImportSummary)
        .iter()
        .all(|e| matches!(e, Effect::Render(_))));
    let mut effects = core.process_event(Event::ConfirmDocumentDeletion(id(1)));
    assert_eq!(core.view().library_delete_error, None);
    finish(&core, &mut effects, StorageResult::DocumentDeleted(id(1)));
    assert_eq!(core.view().library_document_count, 1);
}

#[test]
fn document_deletion_navigation_retires_prompts_and_late_callbacks_only_remove_the_owned_id() {
    for event in [
        Event::OpenSettings,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Vocabulary),
        Event::CloseDocument,
    ] {
        let core = loaded();
        core.process_event(Event::RequestDocumentDeletion(id(1)));
        core.process_event(event);
        assert!(core.view().library_delete_target.is_none());
        assert!(core
            .process_event(Event::ConfirmDocumentDeletion(id(1)))
            .is_empty());
    }
    let core = loaded();
    let mut refresh = core.process_event(Event::RefreshLibrary);
    assert!(core
        .process_event(Event::RequestDocumentDeletion(id(1)))
        .is_empty());
    finish(
        &core,
        &mut refresh,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::RequestDocumentDeletion(id(1)));
    let mut effects = core.process_event(Event::ConfirmDocumentDeletion(id(1)));
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    finish(&core, &mut effects, StorageResult::DocumentDeleted(id(1)));
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert_eq!(core.view().library_document_count, 1);
    let core = loaded();
    core.process_event(Event::RequestDocumentDeletion(id(1)));
    let mut effects = core.process_event(Event::ConfirmDocumentDeletion(id(1)));
    finish(&core, &mut effects, StorageResult::DocumentDeleted(id(2)));
    assert_eq!(core.view().library_document_count, 2);
}
