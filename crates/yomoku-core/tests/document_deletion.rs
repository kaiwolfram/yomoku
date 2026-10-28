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
    loaded_count(2)
}

fn loaded_count(count: i64) -> Core<Yomoku> {
    let core = Core::<Yomoku>::new();
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        &core,
        &mut effects,
        StorageResult::Listed(
            (1..=count)
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
fn library_selection_is_visible_read_only_and_select_all_includes_offscreen_matches() {
    let core = loaded_count(250);
    assert!(core
        .process_event(Event::SelectAllLibraryDocuments)
        .is_empty());
    assert!(core
        .process_event(Event::RequestSelectedDocumentDeletion)
        .is_empty());
    assert!(core
        .process_event(Event::ToggleLibraryDocumentSelection(id(999)))
        .is_empty());
    core.process_event(Event::SearchLibraryTitles("1".into()));
    assert!(core
        .process_event(Event::ToggleLibraryDocumentSelection(id(2)))
        .is_empty());
    let effects = core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
    assert!(matches!(effects.as_slice(), [Effect::Render(_)]));
    assert_eq!(core.view().library_selection, Some(vec![id(1)]));
    core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
    assert_eq!(core.view().library_selection, Some(vec![]));
    core.process_event(Event::SelectAllLibraryDocuments);
    let expected: Vec<_> = (1..=250)
        .filter(|n| n.to_string().contains('1'))
        .map(id)
        .collect();
    assert!(expected.len() > 100);
    assert_eq!(core.view().library_selection, Some(expected));
    core.process_event(Event::SearchLibraryTitles("2".into()));
    core.process_event(Event::SelectAllLibraryDocuments);
    core.process_event(Event::SelectAllLibraryDocuments);
    let expected: Vec<_> = (1..=250)
        .filter(|n| n.to_string().contains(['1', '2']))
        .map(id)
        .collect();
    assert_eq!(core.view().library_selection, Some(expected));
    core.process_event(Event::SearchLibraryTitles(String::new()));
    core.process_event(Event::SelectAllLibraryDocuments);
    assert_eq!(
        core.view().library_selection,
        Some((1..=250).map(id).collect())
    );
    core.process_event(Event::CancelLibrarySelection);
    assert!(core.view().library_selection.is_none());
    assert_eq!(core.view().library_document_count, 250);
}

#[test]
fn bulk_confirmation_freezes_exact_ids_and_cancel_retains_selection_without_storage() {
    let core = loaded();
    assert!(core
        .process_event(Event::ConfirmSelectedDocumentDeletion(vec![id(1)]))
        .is_empty());
    core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
    assert!(core
        .process_event(Event::RequestDocumentRename(id(1)))
        .is_empty());
    assert!(core
        .process_event(Event::RequestDocumentDeletion(id(1)))
        .is_empty());
    core.process_event(Event::SelectAllLibraryDocuments);
    core.process_event(Event::RequestSelectedDocumentDeletion);
    let targets = core.view().library_bulk_delete_targets.unwrap();
    assert_eq!(
        targets.iter().map(|t| t.document_id).collect::<Vec<_>>(),
        vec![id(1), id(2)]
    );
    core.process_event(Event::SearchLibraryTitles("no matches".into()));
    core.process_event(Event::SortLibrary(yomoku_core::library::LibrarySort::Title));
    assert_eq!(core.view().library_bulk_delete_targets, Some(targets));
    for event in [
        Event::ToggleLibraryDocumentSelection(id(1)),
        Event::SelectAllLibraryDocuments,
        Event::RequestSelectedDocumentDeletion,
        Event::ConfirmSelectedDocumentDeletion(vec![]),
        Event::ConfirmSelectedDocumentDeletion(vec![id(1)]),
        Event::ConfirmSelectedDocumentDeletion(vec![id(2), id(1)]),
        Event::ConfirmSelectedDocumentDeletion(vec![id(1), id(1)]),
    ] {
        assert!(core.process_event(event).is_empty());
    }
    assert!(core
        .process_event(Event::CancelDocumentDeletion)
        .iter()
        .all(|e| matches!(e, Effect::Render(_))));
    assert!(core.view().library_bulk_delete_targets.is_none());
    assert_eq!(core.view().library_selection, Some(vec![id(1), id(2)]));
    assert!(core
        .process_event(Event::ConfirmSelectedDocumentDeletion(vec![id(1), id(2)]))
        .is_empty());
    core.process_event(Event::CancelLibrarySelection);
    assert!(core.view().library_selection.is_none());
}

#[test]
fn bulk_deletion_retries_only_frozen_selection_and_reconciles_full_library() {
    let core = loaded_count(3);
    core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
    core.process_event(Event::ToggleLibraryDocumentSelection(id(3)));
    core.process_event(Event::RequestSelectedDocumentDeletion);
    // Read-only status scans must preserve selection and confirmation.
    let mut scan = core.process_event(Event::LoadLibraryWordDistributions);
    finish(
        &core,
        &mut scan,
        StorageResult::LibraryWordDistributionsLoaded(vec![]),
    );
    assert!(core.view().library_bulk_delete_targets.is_some());
    let confirm = Event::ConfirmSelectedDocumentDeletion(vec![id(1), id(3)]);
    let mut effects = core.process_event(confirm.clone());
    assert!(effects.iter().any(|effect| matches!(effect, Effect::Storage(request) if request.operation == StorageOperation::DeleteDocuments(vec![id(1), id(3)]))));
    for event in [
        confirm.clone(),
        Event::CancelDocumentDeletion,
        Event::CancelLibrarySelection,
        Event::ToggleLibraryDocumentSelection(id(2)),
        Event::SelectAllLibraryDocuments,
    ] {
        assert!(core.process_event(event).is_empty());
    }
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(!core.view().storage_busy);
    assert_eq!(core.view().library_document_count, 3);
    assert_eq!(
        core.view().library_delete_error,
        Some(DocumentError::Unavailable)
    );
    assert_eq!(core.view().document_error, None);
    assert_eq!(core.view().library_selection, Some(vec![id(1), id(3)]));
    core.process_event(Event::SearchLibraryTitles("3".into()));
    core.process_event(Event::SortLibrary(yomoku_core::library::LibrarySort::Title));
    let mut effects = core.process_event(confirm);
    assert!(core.view().library_delete_error.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentsDeleted(vec![id(1), id(3)]),
    );
    let view = core.view();
    assert!(view.library_selection.is_none());
    assert!(view.library_bulk_delete_targets.is_none());
    assert!(view.library_word_distributions.is_none());
    assert!(view.documents.is_empty());
    assert_eq!(view.library_document_count, 1);
    assert_eq!(view.library_title_query, "3");
    assert_eq!(view.library_sort, yomoku_core::library::LibrarySort::Title);
    core.process_event(Event::SearchLibraryTitles(String::new()));
    assert_eq!(core.view().documents[0].document_id, id(2));
}

#[test]
fn bulk_selection_navigation_refresh_and_foreign_results_cannot_delete_other_documents() {
    for event in [
        Event::OpenSettings,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Vocabulary),
        Event::CloseDocument,
        Event::RefreshLibrary,
    ] {
        let core = loaded();
        core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
        core.process_event(Event::RequestSelectedDocumentDeletion);
        core.process_event(event);
        assert!(core.view().library_selection.is_none());
        assert!(core.view().library_bulk_delete_targets.is_none());
        assert!(core
            .process_event(Event::ConfirmSelectedDocumentDeletion(vec![id(1)]))
            .is_empty());
    }
    let core = loaded();
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::ToggleLibraryDocumentSelection(id(1)))
        .is_empty());
    let core = loaded();
    core.process_event(Event::ToggleLibraryDocumentSelection(id(1)));
    core.process_event(Event::RequestSelectedDocumentDeletion);
    let mut effects = core.process_event(Event::ConfirmSelectedDocumentDeletion(vec![id(1)]));
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentsDeleted(vec![id(2)]),
    );
    assert_eq!(core.view().library_document_count, 2);
    let mut effects = core.process_event(Event::ConfirmSelectedDocumentDeletion(vec![id(1)]));
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentsDeleted(vec![id(1)]),
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert_eq!(core.view().documents[0].document_id, id(2));
    assert!(core.view().library_selection.is_none());
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
