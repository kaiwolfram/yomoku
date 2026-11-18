use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Yomoku},
    document::{DocumentId, LibraryDocument},
    library::LibrarySort,
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
                    title: format!("本 {n}"),
                    progress_percent: 75,
                    is_read: true,
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
fn rename_requires_visible_target_keeps_literal_draft_and_cancel_is_read_only() {
    let core = loaded();
    let before = core.view().documents;
    assert!(core
        .process_event(Event::SaveDocumentTitle(id(1)))
        .is_empty());
    assert!(core
        .process_event(Event::RequestDocumentRename(id(999)))
        .is_empty());
    core.process_event(Event::RequestDocumentRename(id(1)));
    assert_eq!(core.view().library_rename.unwrap().title, "本 1");
    let title = "  改題𠮷 か\u{3099}\0\n'  ";
    let effects = core.process_event(Event::EditDocumentTitle {
        document_id: id(1),
        title: title.into(),
    });
    assert!(effects.iter().all(|e| matches!(e, Effect::Render(_))));
    assert!(core
        .process_event(Event::SaveDocumentTitle(id(2)))
        .is_empty());
    assert!(core
        .process_event(Event::EditDocumentTitle {
            document_id: id(2),
            title: "wrong".into()
        })
        .is_empty());
    core.process_event(Event::SortLibrary(LibrarySort::Title));
    assert_eq!(core.view().library_rename.unwrap().title, title);
    core.process_event(Event::CancelDocumentRename);
    assert!(core.view().library_rename.is_none());
    let mut after = core.view().documents;
    after.sort_by_key(|d| d.document_id);
    let mut before = before;
    before.sort_by_key(|d| d.document_id);
    assert_eq!(after, before);
}

#[test]
fn rename_updates_matching_full_library_and_sort_without_changing_progress_or_counts() {
    let core = loaded();
    let mut effects = core.process_event(Event::LoadLibraryWordDistributions);
    finish(
        &core,
        &mut effects,
        StorageResult::LibraryWordDistributionsLoaded(vec![]),
    );
    let before = core.view().documents;
    core.process_event(Event::RequestDocumentRename(id(1)));
    for blank in ["", " \n\t\u{3000}"] {
        core.process_event(Event::EditDocumentTitle {
            document_id: id(1),
            title: blank.into(),
        });
        assert!(core
            .process_event(Event::SaveDocumentTitle(id(1)))
            .is_empty());
    }
    core.process_event(Event::EditDocumentTitle {
        document_id: id(1),
        title: "AAA𠮷".into(),
    });
    let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
    assert!(effects.iter().any(|e| matches!(e, Effect::Storage(r) if r.operation == StorageOperation::RenameDocument { document_id: id(1), title: "AAA𠮷".into() })));
    assert!(core.view().storage_busy);
    for event in [
        Event::SaveDocumentTitle(id(1)),
        Event::CancelDocumentRename,
        Event::RequestDocumentDeletion(id(2)),
        Event::EditDocumentTitle {
            document_id: id(1),
            title: "lost".into(),
        },
    ] {
        assert!(core.process_event(event).is_empty());
    }
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentRenamed {
            document_id: id(1),
            title: "AAA𠮷".into(),
        },
    );
    assert!(!core.view().storage_busy);
    assert_eq!(core.view().documents.len(), 2);
    assert!(core.view().library_rename.is_none());
    assert_eq!(core.view().library_word_distributions, Some(vec![]));
    core.process_event(Event::SortLibrary(LibrarySort::Title));
    let renamed = core.view().documents[0].clone();
    assert_eq!(
        renamed,
        LibraryDocument {
            title: "AAA𠮷".into(),
            ..before[1].clone()
        }
    );
}

#[test]
fn rename_failure_preserves_draft_and_separate_error_until_explicit_retry_or_edit() {
    let core = loaded();
    let mut effects = core.process_event(Event::CreatePaste("".into()));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::EmptyPaste),
    );
    core.process_event(Event::RequestDocumentRename(id(1)));
    core.process_event(Event::EditDocumentTitle {
        document_id: id(1),
        title: "改題".into(),
    });
    let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let draft = core.view().library_rename.unwrap();
    assert_eq!(draft.title, "改題");
    assert_eq!(draft.error, Some(DocumentError::Unavailable));
    assert_eq!(core.view().document_error, Some(DocumentError::EmptyPaste));
    core.process_event(Event::DismissImportSummary);
    assert_eq!(core.view().library_rename.unwrap(), draft);
    let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
    assert_eq!(core.view().library_rename.unwrap().error, None);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::EditDocumentTitle {
        document_id: id(1),
        title: "次の題".into(),
    });
    assert_eq!(core.view().library_rename.unwrap().error, None);
    let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentRenamed {
            document_id: id(1),
            title: "次の題".into(),
        },
    );
    assert!(core.view().library_rename.is_none());
    assert_eq!(core.view().document_error, Some(DocumentError::EmptyPaste));
}

#[test]
fn rename_navigation_retires_editor_and_callbacks_require_exact_id_and_title() {
    for event in [
        Event::OpenSettings,
        Event::OpenPaste,
        Event::CloseDocument,
        Event::SelectDestination(Destination::Vocabulary),
        Event::RequestDocumentDeletion(id(2)),
    ] {
        let core = loaded();
        core.process_event(Event::RequestDocumentRename(id(1)));
        core.process_event(event);
        assert!(core.view().library_rename.is_none());
        assert!(core
            .process_event(Event::SaveDocumentTitle(id(1)))
            .is_empty());
    }
    let core = loaded();
    core.process_event(Event::RequestDocumentRename(id(1)));
    let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    finish(
        &core,
        &mut effects,
        StorageResult::DocumentRenamed {
            document_id: id(1),
            title: "本 1".into(),
        },
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert!(core.view().library_rename.is_none());
    for (document_id, title) in [(id(2), "本 1"), (id(1), "foreign")] {
        let core = loaded();
        let before = core.view().documents;
        core.process_event(Event::RequestDocumentRename(id(1)));
        let mut effects = core.process_event(Event::SaveDocumentTitle(id(1)));
        finish(
            &core,
            &mut effects,
            StorageResult::DocumentRenamed {
                document_id,
                title: title.into(),
            },
        );
        assert_eq!(core.view().documents, before);
    }
}
