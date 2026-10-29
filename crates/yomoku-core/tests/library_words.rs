use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Model, Yomoku},
    document::LibraryDocument,
    library::{LibrarySort, LibraryWordDistribution, WordStatusCounts},
    storage::{DocumentError, StorageOperation, StorageResult},
};

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

fn distribution() -> Vec<LibraryWordDistribution> {
    vec![LibraryWordDistribution {
        document_id: 1.try_into().unwrap(),
        counts: Some(WordStatusCounts {
            new: 3,
            known: 1,
            ..Default::default()
        }),
    }]
}

fn cores() -> [Core<Yomoku>; 2] {
    [
        Core::new(),
        Core::new_with(Yomoku, Model::with_effective_reader()),
    ]
}

fn loaded(core: &Core<Yomoku>) {
    let expected = if core.view().effective_reader_status.is_some() {
        StorageOperation::LoadEffectiveLibraryWordDistributions
    } else {
        StorageOperation::LoadLibraryWordDistributions
    };
    let mut effects = core.process_event(Event::LoadLibraryWordDistributions);
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(request)
        if request.operation == expected)));
    assert!(core.view().storage_busy);
    finish(
        core,
        &mut effects,
        StorageResult::LibraryWordDistributionsLoaded(distribution()),
    );
    assert_eq!(core.view().library_word_distributions, Some(distribution()));
    assert!(!core.view().storage_busy);
}

#[test]
fn library_words_are_core_owned_and_search_sort_settings_preserve_the_snapshot() {
    for core in cores() {
        loaded(&core);
        for event in [
            Event::SearchLibraryTitles("literal".into()),
            Event::SortLibrary(LibrarySort::Title),
            Event::OpenSettings,
            Event::CloseSettings,
        ] {
            core.process_event(event.clone());
            assert_eq!(core.view().library_word_distributions, Some(distribution()));
        }
        assert_eq!(
            Core::<Yomoku>::new().view().library_word_distributions,
            None
        );
    }
}

#[test]
fn library_words_refresh_on_return_import_duplicate_and_library_reload() {
    for core in cores() {
        loaded(&core);
        core.process_event(Event::OpenPaste);
        core.process_event(Event::CloseDocument);
        assert_eq!(core.view().library_word_distributions, None);
        loaded(&core);
        core.process_event(Event::SelectDestination(Destination::Vocabulary));
        core.process_event(Event::SelectDestination(Destination::Library));
        assert_eq!(core.view().library_word_distributions, None);
        let document = LibraryDocument {
            document_id: 1.try_into().unwrap(),
            title: "本".into(),
            progress_percent: 0,
            finished_at_least_once: false,
            last_read_sequence: None,
        };
        for result in [
            StorageResult::Created(document.clone()),
            StorageResult::Existing(document),
        ] {
            loaded(&core);
            let mut effects = core.process_event(Event::CreatePaste("本".into()));
            finish(&core, &mut effects, result);
            assert_eq!(core.view().library_word_distributions, None);
        }
        loaded(&core);
        let mut effects = core.process_event(Event::RefreshLibrary);
        finish(
            &core,
            &mut effects,
            StorageResult::Listed(vec![], Default::default(), Default::default()),
        );
        assert_eq!(core.view().library_word_distributions, None);
    }
}

#[test]
fn library_words_failures_are_separate_and_retries_do_not_replace_other_work() {
    for core in cores() {
        let mut effects = core.process_event(Event::RefreshLibrary);
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        let mut effects = core.process_event(Event::LoadLibraryWordDistributions);
        assert_eq!(core.view().library_word_error, None);
        assert!(core
            .process_event(Event::LoadLibraryWordDistributions)
            .is_empty());
        // Search can still update presentation without taking the outstanding read.
        core.process_event(Event::SearchLibraryTitles("本".into()));
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::InvalidDocument),
        );
        assert!(!core.view().storage_busy);
        assert_eq!(
            core.view().library_word_error,
            Some(DocumentError::InvalidDocument)
        );
        assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
        assert_eq!(core.view().library_title_query, "本");
        loaded(&core);
        assert_eq!(core.view().library_word_error, None);
        let mut refresh = core.process_event(Event::RefreshLibrary);
        assert!(core
            .process_event(Event::LoadLibraryWordDistributions)
            .is_empty());
        finish(
            &core,
            &mut refresh,
            StorageResult::Listed(vec![], Default::default(), Default::default()),
        );
        assert_eq!(core.view().library_word_distributions, None);
    }
}

#[test]
fn library_words_hidden_requests_are_rejected() {
    for event in [
        Event::OpenSettings,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Nostr),
    ] {
        for core in cores() {
            core.process_event(event.clone());
            assert!(core
                .process_event(Event::LoadLibraryWordDistributions)
                .is_empty());
            assert!(!core.view().storage_busy);
        }
    }
}

#[test]
fn library_reads_preserve_literal_rename_and_frozen_bulk_delete_prompts() {
    for core in cores() {
        let document = LibraryDocument {
            document_id: 1.try_into().unwrap(),
            title: "本".into(),
            progress_percent: 75,
            finished_at_least_once: true,
            last_read_sequence: Some(10),
        };
        let mut effects = core.process_event(Event::RefreshLibrary);
        finish(
            &core,
            &mut effects,
            StorageResult::Listed(
                vec![document.clone()],
                Default::default(),
                Default::default(),
            ),
        );
        core.process_event(Event::RequestDocumentRename(document.document_id));
        core.process_event(Event::EditDocumentTitle {
            document_id: document.document_id,
            title: " literal𠮷\0 ".into(),
        });
        let draft = core.view().library_rename;
        loaded(&core);
        assert_eq!(core.view().library_rename, draft);
        core.process_event(Event::CancelDocumentRename);
        core.process_event(Event::ToggleLibraryDocumentSelection(document.document_id));
        core.process_event(Event::RequestSelectedDocumentDeletion);
        let before = core.view();
        let mut effects = core.process_event(Event::LoadLibraryWordDistributions);
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        let after = core.view();
        assert_eq!(after.library_selection, before.library_selection);
        assert_eq!(
            after.library_bulk_delete_targets,
            before.library_bulk_delete_targets
        );
        assert_eq!(after.library_delete_error, before.library_delete_error);
        loaded(&core);
        assert_eq!(
            core.view().library_bulk_delete_targets,
            before.library_bulk_delete_targets
        );
        assert_eq!(core.view().documents, vec![document]);
    }
}
