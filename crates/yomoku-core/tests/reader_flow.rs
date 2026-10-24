use crux_core::Core;
use yomoku_core::{
    app::{ContentRoute, Destination, Effect, Event, ImportOutcome, ImportSummary, Yomoku},
    document::{
        AnalysisId, ChapterId, ChapterText, DocumentId, PastedDocument, SourceId, TextOffset,
    },
    pagination::{PageLength, ReadingPosition},
    reader::{progress_percent, ReaderAnalysis, ReaderPage, ReaderSnapshot},
    storage::{DocumentError, StorageOperation, StorageResult},
};

fn document(text: &str) -> PastedDocument {
    PastedDocument {
        document_id: DocumentId::try_from(1).unwrap(),
        title: "題𠮷".into(),
        reading_position: None,
        chapter: ChapterText {
            source_id: SourceId::try_from(2).unwrap(),
            chapter_id: ChapterId::try_from(3).unwrap(),
            text: text.into(),
        },
    }
}

fn loaded(document: PastedDocument, page_length: PageLength) -> StorageResult {
    StorageResult::Loaded(ReaderSnapshot {
        document,
        page_length,
        analysis: ReaderAnalysis::Missing,
    })
}

#[test]
fn reopening_uses_containing_page_but_keeps_exact_saved_offset_including_eof() {
    let first = format!("{}\n", "𠮷".repeat(500));
    let mut document = document(&format!("{first}次の段落。"));
    for byte in [first.len(), first.len() + 3, document.chapter.text.len()] {
        let position = ReadingPosition {
            source_id: document.chapter.source_id,
            chapter_id: document.chapter.chapter_id,
            offset: TextOffset::at(&document.chapter.text, byte).unwrap(),
        };
        document.reading_position = Some(position);
        let page = ReaderPage::open(&document, PageLength::default()).unwrap();
        assert_eq!(page.page_number, 2);
        assert_eq!(page.text, "次の段落。");
        assert_eq!(page.position, position);
        assert_eq!(i64::from(page.anchor.range.start), first.len() as i64);
    }
}

#[test]
fn position_save_keeps_committed_view_until_success_and_late_results_only_update_library() {
    for close in [false, true] {
        let core = Core::<Yomoku>::new();
        assert!(core
            .process_event(Event::SaveReadingPosition(ReadingPosition {
                source_id: SourceId::try_from(2).unwrap(),
                chapter_id: ChapterId::try_from(3).unwrap(),
                offset: TextOffset::START,
            }))
            .is_empty());
        let mut document = document(&format!("{}\n次の段落。", "𠮷".repeat(500)));
        let mut effects = core.process_event(Event::CreatePaste(document.chapter.text.clone()));
        finish(
            &core,
            &mut effects,
            StorageResult::Created(document.clone()),
        );
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(
            &core,
            &mut effects,
            loaded(document.clone(), PageLength::default()),
        );
        let previous = core.view().reader.unwrap();
        let position = ReadingPosition {
            offset: previous.anchor.range.end,
            ..previous.position
        };
        let mut effects = core.process_event(Event::SaveReadingPosition(position));
        assert_eq!(core.view().reader, Some(previous.clone()));
        assert!(core
            .process_event(Event::SaveReadingPosition(position))
            .is_empty());
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().reader, Some(previous.clone()));
        assert_eq!(core.view().documents[0].progress_percent, 0);
        let mut effects = core.process_event(Event::SaveReadingPosition(position));
        assert!(core.view().document_error.is_none());
        if close {
            core.process_event(Event::CloseDocument);
        }
        document.reading_position = Some(position);
        finish(
            &core,
            &mut effects,
            StorageResult::PositionSaved(document.clone()),
        );
        assert_eq!(
            core.view().documents[0],
            document.library_summary().unwrap()
        );
        if close {
            assert!(core.view().reader.is_none());
            assert_eq!(core.view().content_route, ContentRoute::Main);
        } else {
            assert_eq!(core.view().reader.unwrap().position, position);
            let mut effects = core.process_event(Event::SaveReadingPosition(previous.position));
            document.reading_position = Some(previous.position);
            finish(&core, &mut effects, StorageResult::PositionSaved(document));
            assert_eq!(core.view().documents[0].progress_percent, 0);
            assert_eq!(core.view().reader.unwrap().page_number, 1);
        }
    }
}

#[test]
fn current_progress_has_checked_bounds_and_no_overflow_or_finished_flag() {
    for (offset, total, expected) in [
        (0, 0, 0),
        (0, 7, 0),
        (1, 3, 33),
        (7, 7, 100),
        (i64::MAX - 1, i64::MAX, 99),
        (i64::MAX, i64::MAX, 100),
    ] {
        assert_eq!(progress_percent(offset, total).unwrap(), expected);
    }
    for (offset, total) in [(-1, 1), (0, -1), (2, 1), (1, 0)] {
        assert!(progress_percent(offset, total).is_err());
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
    assert!(core.view().import_summary.is_none());
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
    assert_eq!(
        view.import_summary,
        Some(ImportSummary {
            document_id: view.documents[0].document_id,
            title: "題𠮷".into(),
            outcome: ImportOutcome::Created,
        })
    );
}

#[test]
fn duplicate_summary_preserves_progress_survives_settings_and_refresh_and_can_be_dismissed() {
    let core = Core::<Yomoku>::new();
    let mut document = document("題𠮷");
    document.reading_position = Some(ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: TextOffset::try_from(3).unwrap(),
    });
    let mut effects = core.process_event(Event::CreatePaste(document.chapter.text.clone()));
    assert!(core.view().import_summary.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::Existing(document.clone()),
    );
    let summary = core.view().import_summary.unwrap();
    assert_eq!(summary.outcome, ImportOutcome::Existing);
    assert_eq!(summary.document_id, document.document_id);
    assert_eq!(summary.title, document.title);
    assert_eq!(
        core.view().documents,
        vec![document.library_summary().unwrap()]
    );
    assert!(core.view().reader.is_none());
    assert!(core.view().loaded_document.is_none());
    core.process_event(Event::OpenSettings);
    core.process_event(Event::CloseSettings);
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        &core,
        &mut effects,
        StorageResult::Listed(
            vec![document.library_summary().unwrap()],
            PageLength::default(),
        ),
    );
    assert_eq!(core.view().import_summary, Some(summary));
    for _ in 0..2 {
        core.process_event(Event::DismissImportSummary);
        assert!(core.view().import_summary.is_none());
    }
    assert_eq!(core.view().documents.len(), 1);
    assert!(Core::<Yomoku>::new().view().import_summary.is_none());
}

#[test]
fn leaving_library_or_starting_another_import_clears_the_previous_summary() {
    for next in [
        Event::SelectDestination(Destination::Vocabulary),
        Event::OpenPaste,
        Event::OpenReader(document("本文").document_id),
        Event::CreatePaste("次の本文".into()),
    ] {
        let core = Core::<Yomoku>::new();
        let mut effects = core.process_event(Event::CreatePaste("本文".into()));
        finish(
            &core,
            &mut effects,
            StorageResult::Created(document("本文")),
        );
        assert!(core.view().import_summary.is_some());
        let mut effects = core.process_event(next);
        assert!(core.view().import_summary.is_none());
        if core.view().storage_busy {
            finish(
                &core,
                &mut effects,
                StorageResult::Failed(DocumentError::Unavailable),
            );
            assert!(core.view().import_summary.is_none());
        }
    }
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
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
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
        finish(&core, &mut effects, loaded(document, PageLength::default()));
        assert_eq!(core.view().content_route, route);
        assert!(core.view().reader.is_none());
        assert!(core.view().reader_analysis.is_none());
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
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader.is_none());
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    assert_eq!(core.view().reader.unwrap().total_pages, 1);
}

#[test]
fn settings_reflow_preserves_exact_position_for_all_presets_and_leaves_progress_unchanged() {
    let text = format!("{}終。", "𠮷abc。\r\n".repeat(700));
    for old in PageLength::ALL {
        for new in PageLength::ALL {
            let core = Core::<Yomoku>::new();
            let mut document = document(&text);
            let position = ReadingPosition {
                source_id: document.chapter.source_id,
                chapter_id: document.chapter.chapter_id,
                offset: TextOffset::at(&text, 1217).unwrap(), // inside a page, after a supplementary scalar
            };
            document.reading_position = Some(position);
            let mut effects = core.process_event(Event::RefreshLibrary);
            finish(
                &core,
                &mut effects,
                StorageResult::Listed(vec![document.library_summary().unwrap()], old),
            );
            let mut effects = core.process_event(Event::OpenReader(document.document_id));
            finish(&core, &mut effects, loaded(document.clone(), old));
            let previous = core.view().reader.unwrap();
            let progress = core.view().documents;
            core.process_event(Event::OpenSettings);
            let mut effects = core.process_event(Event::SetPageLength(new));
            if old == new {
                assert!(effects.is_empty());
            } else {
                assert_eq!(core.view().reader, Some(previous));
                assert_eq!(core.view().page_length, Some(old));
                assert!(core.process_event(Event::SetPageLength(old)).is_empty());
                finish(&core, &mut effects, StorageResult::PageLengthSaved(new));
            }
            assert_eq!(core.view().page_length, Some(new));
            assert_eq!(
                core.view().reader,
                Some(ReaderPage::at(&document, position, new).unwrap())
            );
            assert_eq!(core.view().reader.unwrap().position, position);
            assert_eq!(core.view().documents, progress);
            core.process_event(Event::CloseSettings);
            assert_eq!(core.view().content_route, ContentRoute::Reader);
        }
    }
}

#[test]
fn settings_failures_keep_committed_page_and_late_saves_do_not_reopen_routes() {
    let core = Core::<Yomoku>::new();
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::SetPageLength(PageLength::Characters250))
        .is_empty());
    assert_eq!(core.view().page_length, None);
    let document = document(&"字。\n".repeat(1000));
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    assert!(core
        .process_event(Event::SetPageLength(PageLength::Characters250))
        .is_empty());
    core.process_event(Event::OpenSettings);
    let previous = core.view().reader;
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().page_length, Some(PageLength::default()));
    assert_eq!(core.view().reader, previous);
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    assert!(core.view().document_error.is_none());
    core.process_event(Event::CloseSettings);
    core.process_event(Event::CloseDocument);
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters250),
    );
    assert_eq!(core.view().page_length, Some(PageLength::Characters250));
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(!core.view().settings_open);
    assert!(core.view().reader.is_none());
}

#[test]
fn reader_analysis_is_frozen_through_position_settings_and_plain_load_and_cleared_on_exit() {
    for analysis in [
        ReaderAnalysis::Missing,
        ReaderAnalysis::Available(AnalysisId::try_from(i64::MAX).unwrap()),
    ] {
        for exit in [
            Event::CloseDocument,
            Event::OpenPaste,
            Event::SelectDestination(Destination::Vocabulary),
        ] {
            let core = Core::<Yomoku>::new();
            let document = document("𠮷猫。\n");
            let mut effects = core.process_event(Event::OpenReader(document.document_id));
            assert!(core.view().reader_analysis.is_none());
            finish(
                &core,
                &mut effects,
                StorageResult::Loaded(ReaderSnapshot {
                    document: document.clone(),
                    page_length: PageLength::default(),
                    analysis,
                }),
            );
            assert_eq!(core.view().reader_analysis, Some(analysis));
            let mut effects = core.process_event(Event::SaveReadingPosition(
                core.view().reader.unwrap().position,
            ));
            finish(
                &core,
                &mut effects,
                StorageResult::PositionSaved(document.clone()),
            );
            assert_eq!(core.view().reader_analysis, Some(analysis));
            core.process_event(Event::OpenSettings);
            let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
            finish(
                &core,
                &mut effects,
                StorageResult::PageLengthSaved(PageLength::Characters250),
            );
            core.process_event(Event::CloseSettings);
            assert_eq!(core.view().reader_analysis, Some(analysis));
            let mut effects = core.process_event(Event::LoadPaste(document.document_id));
            finish(
                &core,
                &mut effects,
                loaded(document.clone(), PageLength::Characters250),
            );
            assert_eq!(core.view().reader_analysis, Some(analysis));
            core.process_event(exit);
            assert!(core.view().reader_analysis.is_none());
            let mut effects = core.process_event(Event::OpenReader(document.document_id));
            assert!(core.view().reader_analysis.is_none());
            finish(
                &core,
                &mut effects,
                loaded(document, PageLength::Characters250),
            );
            assert_eq!(core.view().reader_analysis, Some(ReaderAnalysis::Missing));
        }
    }
}

fn restore_time(
    core: &Core<Yomoku>,
    milliseconds: i64,
) -> yomoku_core::active_time::SavedReadingTime {
    use yomoku_core::active_time::{PendingReadingTime, ReadingTimeUpdate, SavedReadingTime};
    let reader = core.view().reader.unwrap();
    let saved = SavedReadingTime {
        value: ReadingTimeUpdate {
            document_id: reader.document_id,
            position: reader.position,
            pending: PendingReadingTime::from_millis(milliseconds).unwrap(),
        },
        revision: 5,
    };
    let mut effects = core.process_event(Event::RestoreReadingTime);
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::LoadReadingTime {
            document_id: reader.document_id,
            position: reader.position,
        }
    );
    finish(core, &mut effects, StorageResult::ReadingTimeLoaded(saved));
    saved
}

fn storage_operation(effects: &[Effect]) -> StorageOperation {
    effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Storage(request) => Some(request.operation.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn checkpoints_freeze_retries_and_block_movement_without_losing_committed_state() {
    use yomoku_core::active_time::PendingReadingTime;
    let core = Core::<Yomoku>::new();
    assert!(core.process_event(Event::RestoreReadingTime).is_empty());
    assert!(core
        .process_event(Event::RetryReadingTimeCheckpoint)
        .is_empty());
    let mut document = document(&format!("{}\n次。", "𠮷".repeat(500)));
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
    assert!(core.view().reader_time.is_none());
    let saved = restore_time(&core, 123);
    assert_eq!(core.view().reader_time, Some(saved.value));
    assert!(core.process_event(Event::RestoreReadingTime).is_empty());
    let mut checkpoint = saved.value;
    checkpoint.pending = PendingReadingTime::from_millis(456).unwrap();
    let mut effects = core.process_event(Event::CheckpointReadingTime(checkpoint));
    let frozen = storage_operation(&effects);
    assert_eq!(
        frozen,
        StorageOperation::SaveReadingTime {
            previous: saved,
            checkpoint
        }
    );
    assert!(core.view().reading_time_save_pending);
    assert_eq!(core.view().reader_time, Some(saved.value));
    assert!(core
        .process_event(Event::RetryReadingTimeCheckpoint)
        .is_empty());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    for event in [
        Event::CheckpointReadingTime(checkpoint),
        Event::SaveReadingPosition(saved.position()),
        Event::OpenReader(document.document_id),
        Event::RefreshLibrary,
    ] {
        assert!(core.process_event(event).is_empty());
    }
    assert!(core.view().reading_time_save_pending);
    let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
    assert_eq!(storage_operation(&effects), frozen);
    assert!(core.view().document_error.is_none());
    let next = yomoku_core::active_time::SavedReadingTime {
        value: checkpoint,
        revision: 6,
    };
    finish(&core, &mut effects, StorageResult::ReadingTimeSaved(next));
    assert_eq!(core.view().reader_time, Some(checkpoint));
    assert!(!core.view().reading_time_save_pending);
    // A same-offset save and reflow preserve the duration and revision.
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters250),
    );
    core.process_event(Event::CloseSettings);
    assert_eq!(core.view().reader_time, Some(checkpoint));
    let mut effects = core.process_event(Event::SaveReadingPosition(checkpoint.position));
    document.reading_position = Some(checkpoint.position);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.clone()),
    );
    assert_eq!(core.view().reader_time, Some(checkpoint));
    let destination = ReadingPosition {
        offset: core.view().reader.unwrap().anchor.range.end,
        ..checkpoint.position
    };
    let mut effects = core.process_event(Event::SaveReadingPosition(destination));
    document.reading_position = Some(destination);
    finish(&core, &mut effects, StorageResult::PositionSaved(document));
    assert!(core.view().reader_time.is_none());
    assert!(core
        .process_event(Event::CheckpointReadingTime(checkpoint))
        .is_empty());
}

#[test]
fn checkpoint_updates_reject_foreign_anchors_and_decreasing_durations() {
    use yomoku_core::active_time::PendingReadingTime;
    let core = Core::<Yomoku>::new();
    let document = document("𠮷猫。");
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    let saved = restore_time(&core, 900);
    for value in [
        yomoku_core::active_time::ReadingTimeUpdate {
            document_id: DocumentId::try_from(2).unwrap(),
            ..saved.value
        },
        yomoku_core::active_time::ReadingTimeUpdate {
            position: ReadingPosition {
                offset: TextOffset::try_from(4).unwrap(),
                ..saved.position()
            },
            ..saved.value
        },
        yomoku_core::active_time::ReadingTimeUpdate {
            pending: PendingReadingTime::from_millis(899).unwrap(),
            ..saved.value
        },
    ] {
        assert!(core
            .process_event(Event::CheckpointReadingTime(value))
            .is_empty());
        assert_eq!(core.view().reader_time, Some(saved.value));
        assert!(!core.view().reading_time_save_pending);
    }
}

#[test]
fn cancelled_time_restores_and_late_checkpoint_results_never_attach_to_closed_readers() {
    for exit in [
        Event::CloseDocument,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Vocabulary),
    ] {
        let core = Core::<Yomoku>::new();
        let document = document("𠮷猫。");
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(
            &core,
            &mut effects,
            loaded(document.clone(), PageLength::default()),
        );
        let saved = restore_time(&core, 900);
        core.process_event(exit.clone());
        assert!(core.view().reader_time.is_none());
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(&core, &mut effects, loaded(document, PageLength::default()));
        let mut effects = core.process_event(Event::RestoreReadingTime);
        core.process_event(exit.clone());
        finish(&core, &mut effects, StorageResult::ReadingTimeLoaded(saved));
        assert!(core.view().reader_time.is_none());
    }
    for fail in [false, true] {
        let core = Core::<Yomoku>::new();
        let document = document("𠮷猫。");
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(&core, &mut effects, loaded(document, PageLength::default()));
        let saved = restore_time(&core, 900);
        let mut effects = core.process_event(Event::CheckpointReadingTime(saved.value));
        core.process_event(Event::CloseDocument);
        if fail {
            finish(
                &core,
                &mut effects,
                StorageResult::Failed(DocumentError::Unavailable),
            );
            assert!(core.view().reading_time_save_pending);
            effects = core.process_event(Event::RetryReadingTimeCheckpoint);
        }
        finish(&core, &mut effects, StorageResult::ReadingTimeSaved(saved));
        assert!(core.view().reader_time.is_none());
        assert!(!core.view().reading_time_save_pending);
        assert_eq!(core.view().content_route, ContentRoute::Main);
    }
}
