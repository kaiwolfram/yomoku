use crux_core::Core;
use yomoku_core::{
    app::{Destination, Effect, Event, Yomoku},
    document::{ChapterText, LibraryDocument, PastedDocument},
    library::LibrarySort,
    reader::{DocumentReaderSnapshot, ReaderAnalysis, ReaderChapterAnalysis},
    storage::{DocumentError, StorageOperation, StorageResult},
};

fn documents() -> Vec<LibraryDocument> {
    [
        ("beta", None),
        ("ALPHA", Some(2)),
        ("alpha", Some(1)),
        ("𠮷か\u{3099}\0", None),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (title, sequence))| LibraryDocument {
        document_id: (index as i64 + 1).try_into().unwrap(),
        title: title.into(),
        last_read_sequence: sequence,
        progress_percent: index as u64 * 10,
        is_read: index == 2,
    })
    .collect()
}

fn ids(documents: &[LibraryDocument]) -> Vec<i64> {
    documents
        .iter()
        .map(|document| document.document_id.into())
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
fn sort_orders_recent_readers_then_newest_unopened_and_has_stable_unicode_title_ties() {
    let documents = documents();
    for (sort, expected) in [
        (LibrarySort::LastRead, vec![2, 3, 4, 1]),
        (LibrarySort::Title, vec![3, 2, 1, 4]),
        (LibrarySort::ImportDate, vec![4, 3, 2, 1]),
        (LibrarySort::LastReadAscending, vec![1, 4, 3, 2]),
        (LibrarySort::TitleDescending, vec![4, 1, 2, 3]),
        (LibrarySort::ImportDateAscending, vec![1, 2, 3, 4]),
    ] {
        let projected = sort.project(&documents);
        assert_eq!(ids(&projected), expected);
        for document in projected {
            assert_eq!(
                documents
                    .iter()
                    .find(|old| old.document_id == document.document_id),
                Some(&document)
            );
        }
    }
    assert_eq!(ids(&documents), vec![1, 2, 3, 4]);
    let mut wide = documents.clone();
    wide[0].last_read_sequence = Some(i64::MAX);
    assert_eq!(ids(&LibrarySort::LastRead.project(&wide)), vec![1, 2, 3, 4]);
}

#[test]
fn sort_is_read_only_retained_across_refresh_errors_navigation_and_recreation() {
    let core = Core::<Yomoku>::new();
    load(&core);
    assert_eq!(core.view().library_sort, LibrarySort::LastRead);
    let mut effects = core.process_event(Event::RefreshLibrary);
    let sorting = core.process_event(Event::SortLibrary(LibrarySort::TitleDescending));
    assert!(matches!(sorting.as_slice(), [Effect::Render(_)]));
    assert!(core.view().storage_busy);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(ids(&core.view().documents), vec![4, 1, 2, 3]);
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    for event in [
        Event::OpenSettings,
        Event::CloseSettings,
        Event::SelectDestination(Destination::Vocabulary),
        Event::SelectDestination(Destination::Library),
        Event::OpenPaste,
        Event::CloseDocument,
    ] {
        core.process_event(event);
        assert_eq!(core.view().library_sort, LibrarySort::TitleDescending);
    }
    load(&core);
    assert_eq!(ids(&core.view().documents), vec![4, 1, 2, 3]);
    assert_eq!(
        Core::<Yomoku>::new().view().library_sort,
        LibrarySort::LastRead
    );
}

#[test]
fn hidden_sort_callbacks_cannot_change_session_sort() {
    let core = Core::<Yomoku>::new();
    load(&core);
    core.process_event(Event::SortLibrary(LibrarySort::Title));
    core.process_event(Event::SortLibrary(LibrarySort::TitleDescending));
    for event in [
        Event::OpenSettings,
        Event::SelectDestination(Destination::Statistics),
        Event::OpenPaste,
        Event::OpenReader(1.try_into().unwrap()),
    ] {
        core.process_event(event);
        assert!(core
            .process_event(Event::SortLibrary(LibrarySort::ImportDate))
            .is_empty());
        assert_eq!(core.view().library_sort, LibrarySort::TitleDescending);
    }
}

#[test]
fn set_read_requires_a_visible_row_and_updates_only_after_matching_confirmation() {
    let core = Core::<Yomoku>::new();
    load(&core);
    let before = core.view().documents;
    let document_id = before[0].document_id;
    let change = Event::SetDocumentRead {
        document_id,
        is_read: true,
    };
    assert!(core
        .process_event(Event::SetDocumentRead {
            document_id: 999.try_into().unwrap(),
            is_read: true,
        })
        .is_empty());
    core.process_event(Event::OpenSettings);
    assert!(core.process_event(change.clone()).is_empty());
    core.process_event(Event::CloseSettings);

    let mut pending = core.process_event(change.clone());
    assert!(pending.iter().any(|effect| matches!(effect,
        Effect::Storage(request) if request.operation == StorageOperation::SetDocumentRead {
            document_id,
            is_read: true,
        }
    )));
    assert!(core.view().storage_busy);
    assert_eq!(core.view().documents, before);
    assert!(core.process_event(change.clone()).is_empty());
    assert!(core.process_event(Event::RefreshLibrary).is_empty());
    finish(
        &core,
        &mut pending,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    assert_eq!(core.view().documents, before);

    let mut pending = core.process_event(change.clone());
    finish(
        &core,
        &mut pending,
        StorageResult::DocumentReadSet {
            document_id: before[2].document_id,
            is_read: true,
        },
    );
    assert_eq!(core.view().documents, before);
    let mut pending = core.process_event(change.clone());
    finish(
        &core,
        &mut pending,
        StorageResult::DocumentReadSet {
            document_id,
            is_read: true,
        },
    );
    let mut marked = before.clone();
    marked[0].is_read = true;
    assert_eq!(core.view().documents, marked);
    assert!(core.view().document_error.is_none());
    assert!(core.process_event(change).is_empty());

    let mut pending = core.process_event(Event::SetDocumentRead {
        document_id,
        is_read: false,
    });
    finish(
        &core,
        &mut pending,
        StorageResult::DocumentReadSet {
            document_id,
            is_read: false,
        },
    );
    assert_eq!(core.view().documents, before);
}

#[test]
fn opened_snapshot_recency_survives_position_updates_and_duplicate_import_results() {
    let core = Core::<Yomoku>::new();
    load(&core);
    let document: yomoku_core::document::ReaderDocument = PastedDocument {
        document_id: 1.try_into().unwrap(),
        title: "beta".into(),
        finished_at_least_once: false,
        read_override: Some(true),
        chapter: ChapterText {
            source_id: 1.try_into().unwrap(),
            chapter_id: 1.try_into().unwrap(),
            text: "beta\n本文".into(),
        },
        reading_position: None,
    }
    .into();
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        StorageResult::Loaded(DocumentReaderSnapshot {
            last_read_sequence: Some(3),
            typography: Default::default(),
            document: document.clone(),
            page_length: Default::default(),
            analyses: vec![ReaderChapterAnalysis {
                chapter_id: 1.try_into().unwrap(),
                analysis: ReaderAnalysis::Missing,
            }],
        }),
    );
    assert_eq!(ids(&core.view().documents), vec![1, 2, 3, 4]);
    assert!(core.view().documents[0].is_read);
    let position = core.view().reader.unwrap().position;
    let mut effects = core.process_event(Event::SaveReadingPosition(position));
    finish(&core, &mut effects, StorageResult::PositionSaved(document));
    assert_eq!(core.view().documents[0].last_read_sequence, Some(3));
    assert!(core.view().documents[0].is_read);
    core.process_event(Event::CloseDocument);
    let mut duplicate = core.view().documents[0].clone();
    duplicate.progress_percent = 77;
    let mut effects = core.process_event(Event::CreatePaste("same".into()));
    finish(
        &core,
        &mut effects,
        StorageResult::Existing(duplicate.clone()),
    );
    assert_eq!(core.view().documents[0], duplicate);
    let mut fresh = documents()[0].clone();
    fresh.document_id = 5.try_into().unwrap();
    let mut effects = core.process_event(Event::CreatePaste("new".into()));
    finish(&core, &mut effects, StorageResult::Created(fresh));
    assert_eq!(ids(&core.view().documents), vec![1, 2, 3, 5, 4]);
    core.process_event(Event::SortLibrary(LibrarySort::ImportDate));
    assert_eq!(ids(&core.view().documents), vec![5, 4, 3, 2, 1]);
}
