use crux_core::Core;
use yomoku_core::{
    active_time::{ReaderClockAction, ReaderClockSample, SavedReadingTime},
    app::{ContentRoute, Destination, Effect, Event, ImportOutcome, ImportSummary, Yomoku},
    document::{
        AnalysisId, ChapterId, ChapterText, DocumentId, PastedDocument, SourceId, TextOffset,
    },
    pagination::{PageLength, ReadingPosition},
    reader::{progress_percent, ReaderAnalysis, ReaderPage, ReaderSnapshot},
    storage::{DocumentError, StorageOperation, StorageResult},
};

fn clock(core: &Core<Yomoku>, monotonic_ms: i64, action: ReaderClockAction) -> Event {
    let reader = core.view().reader.unwrap();
    Event::ObserveReaderClock(ReaderClockSample {
        document_id: reader.document_id,
        position: reader.position,
        monotonic_ms,
        action,
    })
}

fn save_clock(core: &Core<Yomoku>, effects: &mut [Effect], expected_ms: i64) -> Vec<Effect> {
    let StorageOperation::SaveReadingTime {
        previous,
        checkpoint,
    } = storage_operation(effects)
    else {
        panic!("clock must save an absolute duration");
    };
    assert_eq!(checkpoint.pending.milliseconds(), expected_ms);
    resolve(
        core,
        effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: checkpoint,
            revision: previous.revision + 1,
        }),
    )
}

#[test]
fn reader_clock_requires_restore_rejects_foreign_and_invalid_samples_and_saves_only_durations() {
    let (core, _) = analyzed_reader();
    assert!(core
        .process_event(clock(&core, 0, ReaderClockAction::Resume))
        .is_empty());
    let saved = restore_time(&core, 123);
    assert!(core
        .process_event(clock(&core, 1_000, ReaderClockAction::Resume))
        .is_empty());
    let Event::ObserveReaderClock(sample) = clock(&core, 5_000, ReaderClockAction::Pause) else {
        unreachable!();
    };
    for invalid in [
        ReaderClockSample {
            monotonic_ms: -1,
            ..sample
        },
        ReaderClockSample {
            monotonic_ms: 999,
            ..sample
        },
        ReaderClockSample {
            document_id: DocumentId::try_from(99).unwrap(),
            ..sample
        },
        ReaderClockSample {
            position: ReadingPosition {
                offset: TextOffset::try_from(4).unwrap(),
                ..sample.position
            },
            ..sample
        },
    ] {
        assert!(core
            .process_event(Event::ObserveReaderClock(invalid))
            .is_empty());
        assert!(!core.view().reading_time_save_pending);
    }
    let mut effects = core.process_event(clock(&core, 2_000, ReaderClockAction::Checkpoint));
    assert_eq!(core.view().reader_time, Some(saved.value));
    // A delayed save callback does not discard the accumulator's newer interval.
    core.process_event(clock(&core, 2_500, ReaderClockAction::Checkpoint));
    core.process_event(clock(&core, 2_600, ReaderClockAction::Interact));
    let mut chained = save_clock(&core, &mut effects, 1_123);
    save_clock(&core, &mut chained, 1_623);
    let mut effects = core.process_event(clock(&core, 3_000, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 2_123);
    assert!(core
        .process_event(clock(&core, 90_000, ReaderClockAction::Checkpoint))
        .is_empty());
    assert_eq!(
        core.view().reader_time.unwrap().pending.milliseconds(),
        2_123
    );
}

#[test]
fn reader_clock_caps_idle_and_interactions_resume_without_backfilling_paused_or_idle_time() {
    let (core, _) = analyzed_reader();
    restore_time(&core, 50);
    core.process_event(clock(&core, 10_000, ReaderClockAction::Resume));
    core.process_event(clock(&core, 200_000, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 900_000, ReaderClockAction::Checkpoint));
    save_clock(&core, &mut effects, 300_050);
    core.process_event(clock(&core, 1_000_000, ReaderClockAction::Interact));
    let mut effects = core.process_event(clock(&core, 1_001_000, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 301_050);
    core.process_event(clock(&core, 2_000_000, ReaderClockAction::Interact));
    assert!(core
        .process_event(clock(&core, 2_001_000, ReaderClockAction::Pause))
        .is_empty());
    core.process_event(clock(&core, 3_000_000, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 3_000_500, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 301_550);
}

#[test]
fn final_clock_checkpoint_waits_for_unrelated_work_and_drains_before_first_departure() {
    for failed in [false, true] {
        let (core, _) = analyzed_reader();
        let saved = restore_time(&core, 200);
        core.process_event(clock(&core, 0, ReaderClockAction::Resume));
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        core.process_event(clock(&core, 100, ReaderClockAction::Checkpoint));
        core.process_event(clock(&core, 250, ReaderClockAction::Pause));
        assert!(core.view().reading_time_save_pending);
        core.process_event(Event::CloseDocument);
        core.process_event(Event::OpenPaste);
        assert_eq!(core.view().content_route, ContentRoute::Reader);
        assert!(core
            .process_event(clock(&core, 999, ReaderClockAction::Resume))
            .is_empty());
        let mut effects = resolve(
            &core,
            &mut unrelated,
            if failed {
                StorageResult::Failed(DocumentError::Unavailable)
            } else {
                StorageResult::Listed(core.view().documents, PageLength::default())
            },
        );
        if failed {
            assert!(core.view().reading_time_save_pending && !core.view().storage_busy);
            assert_eq!(core.view().reader_time, Some(saved.value));
            assert!(core.process_event(Event::RefreshLibrary).is_empty());
            // Before a write has started, a newer absolute checkpoint can
            // supersede the waiting slot. Never chain the older value after it.
            effects = core.process_event(Event::CheckpointReadingTime(
                yomoku_core::active_time::ReadingTimeUpdate {
                    pending: yomoku_core::active_time::PendingReadingTime::from_millis(600)
                        .unwrap(),
                    ..saved.value
                },
            ));
        }
        let frozen = storage_operation(&effects);
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
        assert_eq!(storage_operation(&effects), frozen);
        save_clock(&core, &mut effects, if failed { 600 } else { 450 });
        assert_eq!(core.view().content_route, ContentRoute::Main);
        assert!(!core.view().reading_time_save_pending);
    }
}

#[test]
fn clock_checkpoint_queued_behind_unrelated_work_precedes_frozen_completion_even_after_failure() {
    for failed in [false, true] {
        let (core, _) = analyzed_reader();
        restore_time(&core, 100);
        core.process_event(clock(&core, 1_000, ReaderClockAction::Resume));
        let mut unrelated = core.process_event(Event::RefreshLibrary);
        core.process_event(clock(&core, 2_000, ReaderClockAction::Pause));
        let completion = forward(&core, 42);
        core.process_event(completion);
        assert!(core.view().completion_save_pending && core.view().reading_time_save_pending);
        assert!(core
            .process_event(clock(&core, 5_000, ReaderClockAction::Resume))
            .is_empty());
        let mut effects = resolve(
            &core,
            &mut unrelated,
            if failed {
                StorageResult::Failed(DocumentError::Unavailable)
            } else {
                StorageResult::Listed(core.view().documents, PageLength::default())
            },
        );
        if failed {
            assert!(core.process_event(Event::RetryReaderCompletion).is_empty());
            effects = core.process_event(Event::RetryReadingTimeCheckpoint);
        }
        let effects = save_clock(&core, &mut effects, 1_100);
        assert!(
            matches!(storage_operation(&effects), StorageOperation::PreparePageCompletion(intent)
            if intent.completed_at_unix_ms == 42)
        );
        assert!(!core.view().reading_time_save_pending && core.view().completion_save_pending);
    }
}

#[test]
fn reader_clock_survives_reflow_but_movement_exit_and_completion_retire_its_epoch() {
    let (core, mut document) = analyzed_reader();
    let saved = restore_time(&core, 100);
    core.process_event(clock(&core, 9_000, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 10_000, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 1_100);
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(clock(&core, 10_100, ReaderClockAction::Resume))
        .is_empty());
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters250),
    );
    core.process_event(Event::CloseSettings);
    core.process_event(clock(&core, 11_000, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 11_500, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 1_600);
    let next = ReadingPosition {
        offset: core.view().reader.unwrap().anchor.range.end,
        ..saved.position()
    };
    let stale = clock(&core, 12_000, ReaderClockAction::Resume);
    let mut effects = core.process_event(Event::SaveReadingPosition(next));
    // The host must pause before moving. A late old-offset producer cannot queue
    // an unsaveable checkpoint behind an already accepted position change.
    assert!(core
        .process_event(clock(&core, 12_000, ReaderClockAction::Pause))
        .is_empty());
    assert!(core
        .process_event(Event::CheckpointReadingTime(
            yomoku_core::active_time::ReadingTimeUpdate {
                pending: yomoku_core::active_time::PendingReadingTime::from_millis(9_999).unwrap(),
                ..core.view().reader_time.unwrap()
            }
        ))
        .is_empty());
    assert!(!core.view().reading_time_save_pending);
    document.reading_position = Some(next);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.clone()),
    );
    assert!(core.process_event(stale).is_empty());
    restore_time(&core, 20);
    // A new offset accepts a fresh monotonic epoch and cannot inherit old time.
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 40, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 60);
    let mut effects = core.process_event(forward(&core, 42));
    let payload = prepared(storage_operation(&effects));
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::PageCompletionPrepared(payload),
    );
    document.reading_position = Some(payload.completion.next_position(&document).unwrap());
    finish(
        &core,
        &mut effects,
        StorageResult::PageCompleted(
            document.clone(),
            yomoku_core::completion::AnalyzedCompletionOutcome::AlreadyRecorded,
        ),
    );
    restore_time(&core, 0);
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 10, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 10);
    core.process_event(Event::CloseDocument);
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        loaded(document, PageLength::Characters250),
    );
    restore_time(&core, 10);
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 5, ReaderClockAction::Pause));
    save_clock(&core, &mut effects, 15);
}

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
    resolve(core, effects, result);
}

fn resolve(core: &Core<Yomoku>, effects: &mut [Effect], result: StorageResult) -> Vec<Effect> {
    let request = effects
        .iter_mut()
        .find_map(|effect| match effect {
            Effect::Storage(request) => Some(request),
            _ => None,
        })
        .unwrap();
    core.resolve(request, result).unwrap()
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
fn checkpoints_freeze_retries_and_preserve_committed_state_through_reflow_and_movement() {
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
fn cancelled_time_restores_stay_detached_and_checkpoint_exit_waits_through_retries() {
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

#[test]
fn busy_checkpoints_coalesce_maximum_duration_and_retry_each_frozen_revision_before_exit() {
    use yomoku_core::active_time::{PendingReadingTime, SavedReadingTime};
    let core = Core::<Yomoku>::new();
    let document = document("𠮷猫。");
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    let initial = restore_time(&core, 100);
    let update = |ms| yomoku_core::active_time::ReadingTimeUpdate {
        pending: PendingReadingTime::from_millis(ms).unwrap(),
        ..initial.value
    };
    let mut effects = core.process_event(Event::CheckpointReadingTime(update(200)));
    let frozen = storage_operation(&effects);
    for ms in [300, 500, 400, 500, 200, 100] {
        let effects = core.process_event(Event::CheckpointReadingTime(update(ms)));
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::Storage(_))));
    }
    // A larger duration at a foreign anchor must not replace the coalesced save.
    core.process_event(Event::CheckpointReadingTime(
        yomoku_core::active_time::ReadingTimeUpdate {
            document_id: DocumentId::try_from(99).unwrap(),
            ..update(10_000)
        },
    ));
    core.process_event(Event::CloseDocument);
    assert_eq!(core.view().content_route, ContentRoute::Reader);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(!core.view().storage_busy);
    assert!(core.view().reading_time_save_pending);
    assert_eq!(core.view().reader_time, Some(initial.value));
    // A final duration can still arrive after the failed periodic save.
    core.process_event(Event::CheckpointReadingTime(update(600)));
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
    assert_eq!(storage_operation(&effects), frozen);
    let first_saved = SavedReadingTime {
        value: update(200),
        revision: initial.revision + 1,
    };
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(first_saved),
    );
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::SaveReadingTime {
            previous: first_saved,
            checkpoint: update(600),
        }
    );
    assert!(core.view().storage_busy && core.view().reading_time_save_pending);
    assert_eq!(core.view().reader_time, Some(first_saved.value));
    assert_eq!(core.view().content_route, ContentRoute::Reader);
    let second_frozen = storage_operation(&effects);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
    assert_eq!(storage_operation(&effects), second_frozen);
    let effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: update(600),
            revision: first_saved.revision + 1,
        }),
    );
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(_))));
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader_time.is_none());
    assert!(!core.view().storage_busy && !core.view().reading_time_save_pending);
}

#[test]
fn position_waits_for_every_checkpoint_and_duplicate_departures_keep_first_destination() {
    use yomoku_core::active_time::{PendingReadingTime, SavedReadingTime};
    let core = Core::<Yomoku>::new();
    let mut document = document(&format!("{}\n次。", "𠮷".repeat(500)));
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
    let initial = restore_time(&core, 0);
    let first = core.view().reader.unwrap();
    let destination = ReadingPosition {
        offset: first.anchor.range.end,
        ..first.position
    };
    let mut checkpoint = initial.value;
    checkpoint.pending = PendingReadingTime::from_millis(100).unwrap();
    let mut effects = core.process_event(Event::CheckpointReadingTime(checkpoint));
    core.process_event(Event::SaveReadingPosition(destination));
    for event in [
        Event::SaveReadingPosition(destination),
        Event::SaveReadingPosition(first.position),
    ] {
        assert!(core.process_event(event).is_empty());
    }
    checkpoint.pending = PendingReadingTime::from_millis(200).unwrap();
    core.process_event(Event::CheckpointReadingTime(checkpoint));
    let saved = SavedReadingTime {
        value: yomoku_core::active_time::ReadingTimeUpdate {
            pending: PendingReadingTime::from_millis(100).unwrap(),
            ..checkpoint
        },
        revision: 6,
    };
    let mut effects = resolve(&core, &mut effects, StorageResult::ReadingTimeSaved(saved));
    // Another periodic update arrives while the automatically chained save runs.
    checkpoint.pending = PendingReadingTime::from_millis(300).unwrap();
    core.process_event(Event::CheckpointReadingTime(checkpoint));
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: yomoku_core::active_time::ReadingTimeUpdate {
                pending: PendingReadingTime::from_millis(200).unwrap(),
                ..checkpoint
            },
            revision: 7,
        }),
    );
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::SaveReadingTime {
            previous: SavedReadingTime {
                value: yomoku_core::active_time::ReadingTimeUpdate {
                    pending: PendingReadingTime::from_millis(200).unwrap(),
                    ..checkpoint
                },
                revision: 7,
            },
            checkpoint,
        }
    );
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: checkpoint,
            revision: 8,
        }),
    );
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::SaveReadingPosition {
            document_id: document.document_id,
            position: destination,
        }
    );
    assert_eq!(core.view().reader, Some(first));
    assert_eq!(core.view().reader_time, Some(checkpoint));
    document.reading_position = Some(destination);
    finish(&core, &mut effects, StorageResult::PositionSaved(document));
    assert_eq!(core.view().reader.unwrap().position, destination);
    assert!(core.view().reader_time.is_none());
    assert!(!core.view().reading_time_save_pending);
}

#[test]
fn all_reader_exit_routes_wait_for_checkpoints_and_identical_updates_do_not_chain_writes() {
    for exit in [
        Event::CloseDocument,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Vocabulary),
    ] {
        let core = Core::<Yomoku>::new();
        let document = document("𠮷猫。");
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(&core, &mut effects, loaded(document, PageLength::default()));
        let saved = restore_time(&core, 123);
        let mut effects = core.process_event(Event::CheckpointReadingTime(saved.value));
        for _ in 0..3 {
            assert!(core
                .process_event(Event::CheckpointReadingTime(saved.value))
                .is_empty());
        }
        core.process_event(exit.clone());
        assert!(core.view().reader.is_some());
        let effects = resolve(&core, &mut effects, StorageResult::ReadingTimeSaved(saved));
        assert!(!effects
            .iter()
            .any(|effect| matches!(effect, Effect::Storage(_))));
        assert!(core.view().reader.is_none());
        assert!(core.view().reader_analysis.is_none());
        assert!(core.view().reader_time.is_none());
        assert!(!core.view().reading_time_save_pending);
        match exit {
            Event::OpenPaste => assert_eq!(core.view().content_route, ContentRoute::Paste),
            Event::SelectDestination(destination) => {
                assert_eq!(core.view().destination, destination)
            }
            _ => assert_eq!(core.view().content_route, ContentRoute::Main),
        }
    }
}

fn analyzed_reader() -> (Core<Yomoku>, PastedDocument) {
    let core = Core::<Yomoku>::new();
    let document = document(&format!("{}\n猫。", "𠮷".repeat(500)));
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
        StorageResult::Loaded(ReaderSnapshot {
            document: document.clone(),
            page_length: PageLength::default(),
            analysis: ReaderAnalysis::Available(AnalysisId::try_from(42).unwrap()),
        }),
    );
    (core, document)
}

fn forward(core: &Core<Yomoku>, completed_at_unix_ms: i64) -> Event {
    let reader = core.view().reader.unwrap();
    Event::CompleteReaderPage {
        document_id: reader.document_id,
        page: reader.anchor,
        position: reader.position,
        completed_at_unix_ms,
    }
}

fn prepared(operation: StorageOperation) -> yomoku_core::completion::AnalyzedPageCompletion {
    use yomoku_core::{
        completion::{AnalyzedPageCompletion, PageCompletion},
        document::CompletionCommandId,
    };
    let StorageOperation::PreparePageCompletion(intent) = operation else {
        panic!("expected preparation")
    };
    AnalyzedPageCompletion {
        completion: PageCompletion {
            command_id: CompletionCommandId::try_from(99).unwrap(),
            document_id: intent.document_id,
            page: intent.page,
            page_length: intent.page_length,
            departing_position: intent.departing_position,
            completed_at_unix_ms: intent.completed_at_unix_ms,
            pending_time: yomoku_core::active_time::PendingReadingTime::from_millis(900).unwrap(),
        },
        analysis_id: intent.analysis_id,
    }
}

#[test]
fn analyzed_completion_drains_checkpoints_and_freezes_both_retry_stages_before_exit() {
    use yomoku_core::{
        active_time::{PendingReadingTime, SavedReadingTime},
        completion::AnalyzedCompletionOutcome,
    };
    let (core, mut document) = analyzed_reader();
    let saved = restore_time(&core, 10);
    let update = |ms| yomoku_core::active_time::ReadingTimeUpdate {
        pending: PendingReadingTime::from_millis(ms).unwrap(),
        ..saved.value
    };
    let mut effects = core.process_event(Event::CheckpointReadingTime(update(100)));
    core.process_event(Event::CheckpointReadingTime(update(200)));
    let request = forward(&core, 1234);
    core.process_event(request.clone());
    assert!(core.view().completion_save_pending);
    assert!(core.process_event(forward(&core, 9999)).is_empty());
    core.process_event(Event::CloseDocument);
    core.process_event(Event::OpenPaste); // First departure remains close.
    core.process_event(Event::CheckpointReadingTime(update(900)));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(core.process_event(Event::RetryReaderCompletion).is_empty());
    let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
    let first_saved = SavedReadingTime {
        value: update(100),
        revision: saved.revision + 1,
    };
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(first_saved),
    );
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::SaveReadingTime {
            previous: first_saved,
            checkpoint: update(900)
        }
    );
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: update(900),
            revision: first_saved.revision + 1,
        }),
    );
    let frozen_intent = storage_operation(&effects);
    let payload = prepared(frozen_intent.clone());
    assert_eq!(payload.completion.completed_at_unix_ms, 1234);
    assert_eq!(payload.analysis_id, AnalysisId::try_from(42).unwrap());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(core.process_event(Event::RefreshLibrary).is_empty());
    assert!(core
        .process_event(Event::CheckpointReadingTime(update(1000)))
        .is_empty());
    let mut effects = core.process_event(Event::RetryReaderCompletion);
    assert_eq!(storage_operation(&effects), frozen_intent);
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::PageCompletionPrepared(payload),
    );
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::CompletePage(payload)
    );
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let mut effects = core.process_event(Event::RetryReaderCompletion);
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::CompletePage(payload)
    );
    assert!(core.process_event(request).is_empty());
    document.reading_position = Some(payload.completion.next_position(&document).unwrap());
    finish(
        &core,
        &mut effects,
        StorageResult::PageCompleted(
            document,
            AnalyzedCompletionOutcome::Recorded { newly_known: 2 },
        ),
    );
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader.is_none() && core.view().reader_time.is_none());
    assert!(!core.view().completion_save_pending && !core.view().reading_time_save_pending);
    assert!(core.view().completion_notice.is_none()); // Exit retires the transient notice.
}

#[test]
fn analyzed_completion_notifies_only_new_success_and_rejects_old_page_callbacks() {
    use yomoku_core::completion::AnalyzedCompletionOutcome;
    for outcome in [
        AnalyzedCompletionOutcome::Recorded { newly_known: 2 },
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 },
        AnalyzedCompletionOutcome::AlreadyRecorded,
    ] {
        let (core, mut document) = analyzed_reader();
        let request = forward(&core, 555);
        let mut effects = core.process_event(request.clone());
        let payload = prepared(storage_operation(&effects));
        let mut effects = resolve(
            &core,
            &mut effects,
            StorageResult::PageCompletionPrepared(payload),
        );
        document.reading_position = Some(payload.completion.next_position(&document).unwrap());
        finish(
            &core,
            &mut effects,
            StorageResult::PageCompleted(document.clone(), outcome),
        );
        assert_eq!(
            core.view().reader.as_ref().unwrap().position,
            document.reading_position.unwrap()
        );
        assert_eq!(
            core.view().documents[0].progress_percent,
            document.library_summary().unwrap().progress_percent
        );
        assert!(!core.view().completion_save_pending && core.view().reader_time.is_none());
        assert_eq!(
            core.view()
                .completion_notice
                .map(|notice| (notice.command_id, notice.newly_known)),
            if outcome == (AnalyzedCompletionOutcome::Recorded { newly_known: 2 }) {
                Some((payload.completion.command_id, 2))
            } else {
                None
            }
        );
        assert!(core.process_event(request).is_empty());
        assert!(core.process_event(Event::RetryReaderCompletion).is_empty());
        core.process_event(Event::DismissCompletionNotice);
        assert!(core.view().completion_notice.is_none());
        // Position-only backward movement enables a fresh explicit reread intent.
        document.reading_position = Some(payload.completion.departing_position);
        let mut effects = core.process_event(Event::SaveReadingPosition(
            payload.completion.departing_position,
        ));
        finish(&core, &mut effects, StorageResult::PositionSaved(document));
        let effects = core.process_event(forward(&core, 777));
        assert!(
            matches!(storage_operation(&effects), StorageOperation::PreparePageCompletion(intent) if intent.completed_at_unix_ms == 777)
        );
    }
}

#[test]
fn completion_requires_matching_visible_page_and_explicit_stored_analysis() {
    let (core, document) = analyzed_reader();
    let mut invalid = core.view().reader.unwrap();
    invalid.position.offset = invalid.anchor.range.end;
    assert!(core
        .process_event(Event::CompleteReaderPage {
            document_id: invalid.document_id,
            page: invalid.anchor,
            position: invalid.position,
            completed_at_unix_ms: 1
        })
        .is_empty());
    assert!(core.process_event(forward(&core, -1)).is_empty());
    let mut busy = core.process_event(Event::RestoreReadingTime);
    assert!(core.process_event(forward(&core, 1)).is_empty());
    assert!(!core.view().completion_save_pending);
    finish(
        &core,
        &mut busy,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::CloseDocument);
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    let effects = core.process_event(forward(&core, 1));
    assert!(!effects
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(_))));
    assert_eq!(
        core.view().document_error,
        Some(DocumentError::MissingAnalysis)
    );
    assert!(!core.view().completion_save_pending);
}
