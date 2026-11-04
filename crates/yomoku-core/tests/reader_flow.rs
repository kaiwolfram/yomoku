use crux_core::Core;
use yomoku_core::{
    active_time::{ReaderClockAction, ReaderClockSample, SavedReadingTime},
    app::{ContentRoute, Destination, Effect, Event, ImportOutcome, ImportSummary, Yomoku},
    document::{
        AnalysisId, ChapterId, ChapterText, DocumentId, PastedDocument, SourceId, TextOffset,
    },
    pagination::{PageLength, ReadingPosition},
    reader::{
        progress_percent, DocumentReaderSnapshot, ReaderAnalysis, ReaderChapterAnalysis, ReaderPage,
    },
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

fn snapshot(
    document: PastedDocument,
    page_length: PageLength,
    analysis: ReaderAnalysis,
) -> StorageResult {
    StorageResult::Loaded(DocumentReaderSnapshot {
        last_read_sequence: None,
        typography: Default::default(),
        analyses: vec![ReaderChapterAnalysis {
            chapter_id: document.chapter.chapter_id,
            analysis,
        }],
        document: document.into(),
        page_length,
    })
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
                StorageResult::Listed(
                    core.view().documents,
                    PageLength::default(),
                    Default::default(),
                )
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

fn document(text: &str) -> PastedDocument {
    PastedDocument {
        document_id: DocumentId::try_from(1).unwrap(),
        title: "題𠮷".into(),
        finished_at_least_once: false,
        reading_position: None,
        chapter: ChapterText {
            source_id: SourceId::try_from(2).unwrap(),
            chapter_id: ChapterId::try_from(3).unwrap(),
            text: text.into(),
        },
    }
}

fn loaded(document: PastedDocument, page_length: PageLength) -> StorageResult {
    snapshot(document, page_length, ReaderAnalysis::Missing)
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
            StorageResult::Created(document.clone().library_summary().unwrap()),
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
            StorageResult::PositionSaved(document.clone().into()),
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
            finish(
                &core,
                &mut effects,
                StorageResult::PositionSaved(document.into()),
            );
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
fn leaving_library_or_starting_another_import_clears_the_previous_summary() {
    for next in [
        Event::SelectDestination(Destination::Vocabulary),
        Event::OpenPaste,
        Event::OpenReader(document("本文").document_id),
        Event::CreatePaste("次の本文".into()),
        Event::ImportTxt(yomoku_core::imports::TxtImport {
            original_bytes: b"text".to_vec(),
            filename: "new.txt".into(),
            encoding: None,
        }),
        Event::ImportSrt(yomoku_core::imports::SrtImport {
            original_bytes: b"text".to_vec(),
            filename: "new.srt".into(),
            encoding: None,
        }),
    ] {
        let core = Core::<Yomoku>::new();
        let mut effects = core.process_event(Event::CreatePaste("本文".into()));
        finish(
            &core,
            &mut effects,
            StorageResult::Created(document("本文").library_summary().unwrap()),
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
        Effect::Storage(request) if request.operation == StorageOperation::OpenReader(document.document_id))));
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
                StorageResult::Listed(
                    vec![document.library_summary().unwrap()],
                    old,
                    Default::default(),
                ),
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

#[test]
fn cancelled_txt_clears_busy_without_an_error_summary_or_reader_movement() {
    let (core, _) = analyzed_reader();
    let before = core.view();
    let mut effects = core.process_event(Event::ImportTxt(yomoku_core::imports::TxtImport {
        original_bytes: b"cancelled".to_vec(),
        filename: "cancelled.txt".into(),
        encoding: None,
    }));
    assert!(core.view().storage_busy);
    finish(&core, &mut effects, StorageResult::ImportCancelled);
    let after = core.view();
    assert!(!after.storage_busy);
    assert!(after.document_error.is_none() && after.import_summary.is_none());
    assert_eq!(after.documents, before.documents);
    assert_eq!(after.reader, before.reader);
    assert_eq!(after.destination, before.destination);
    assert_eq!(after.content_route, before.content_route);
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
        StorageResult::PositionSaved(document.clone().into()),
    );
    assert_eq!(core.view().reader_time, Some(checkpoint));
    let destination = ReadingPosition {
        offset: core.view().reader.unwrap().anchor.range.end,
        ..checkpoint.position
    };
    let mut effects = core.process_event(Event::SaveReadingPosition(destination));
    document.reading_position = Some(destination);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.into()),
    );
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
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.into()),
    );
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
        StorageResult::Created(document.clone().library_summary().unwrap()),
    );
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        snapshot(
            document.clone(),
            PageLength::default(),
            ReaderAnalysis::Available(AnalysisId::try_from(42).unwrap()),
        ),
    );
    (core, document)
}

#[test]
fn previous_is_anchored_position_only_and_drains_final_time_before_moving() {
    let (core, mut document) = analyzed_reader();
    let first = core.view().reader.unwrap();
    let previous = |reader: &ReaderPage| Event::PreviousReaderPage {
        document_id: reader.document_id,
        page: reader.anchor,
        position: reader.position,
    };
    assert!(core.process_event(previous(&first)).is_empty());
    let second_position = ReadingPosition {
        offset: first.anchor.range.end,
        ..first.position
    };
    let mut movement = core.process_event(Event::SaveReadingPosition(second_position));
    document.reading_position = Some(second_position);
    finish(
        &core,
        &mut movement,
        StorageResult::PositionSaved(document.clone().into()),
    );
    let second = core.view().reader.unwrap();
    assert_eq!(second.page_number, 2);
    assert!(core.process_event(previous(&first)).is_empty());
    restore_time(&core, 0);
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut checkpoint = core.process_event(clock(&core, 100, ReaderClockAction::Pause));
    let deferred = core.process_event(previous(&second));
    assert!(!deferred
        .iter()
        .any(|effect| matches!(effect, Effect::Storage(_))));
    assert!(core.process_event(previous(&second)).is_empty());
    let mut movement = save_clock(&core, &mut checkpoint, 100);
    assert_eq!(
        storage_operation(&movement),
        StorageOperation::SaveReadingPosition {
            document_id: document.document_id,
            position: first.position,
        }
    );
    assert!(core.process_event(previous(&second)).is_empty());
    document.reading_position = Some(first.position);
    finish(
        &core,
        &mut movement,
        StorageResult::PositionSaved(document.into()),
    );
    assert_eq!(core.view().reader.unwrap().position, first.position);
    assert!(core.view().reader_time.is_none());
    assert!(core.view().completion_notice.is_none());
    assert!(core.process_event(previous(&second)).is_empty());
}

#[test]
fn previous_ignores_foreign_and_settings_callbacks() {
    let (core, mut document) = analyzed_reader();
    let first = core.view().reader.unwrap();
    let position = ReadingPosition {
        offset: first.anchor.range.end,
        ..first.position
    };
    let mut effects = core.process_event(Event::SaveReadingPosition(position));
    document.reading_position = Some(position);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.into()),
    );
    let second = core.view().reader.unwrap();
    assert!(core
        .process_event(Event::PreviousReaderPage {
            document_id: DocumentId::try_from(99).unwrap(),
            page: second.anchor,
            position: second.position,
        })
        .is_empty());
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::PreviousReaderPage {
            document_id: second.document_id,
            page: second.anchor,
            position: second.position,
        })
        .is_empty());
}

#[test]
fn cancelled_srt_clears_busy_without_an_error_summary_or_reader_movement() {
    let (core, _) = analyzed_reader();
    let before = core.view();
    let mut effects = core.process_event(Event::ImportSrt(yomoku_core::imports::SrtImport {
        original_bytes: b"cancelled".to_vec(),
        filename: "cancelled.srt".into(),
        encoding: None,
    }));
    assert!(core.view().storage_busy);
    finish(&core, &mut effects, StorageResult::ImportCancelled);
    let after = core.view();
    assert!(!after.storage_busy);
    assert!(after.document_error.is_none() && after.import_summary.is_none());
    assert_eq!(after.documents, before.documents);
    assert_eq!(after.reader, before.reader);
    assert_eq!(after.destination, before.destination);
    assert_eq!(after.content_route, before.content_route);
}

#[test]
fn cancelled_epub_clears_busy_without_an_error_summary_or_reader_movement() {
    let (core, _) = analyzed_reader();
    let before = core.view();
    let mut effects = core.process_event(Event::ImportEpub(yomoku_core::imports::EpubImport {
        original_bytes: b"cancelled".to_vec(),
        filename: "cancelled.epub".into(),
    }));
    assert!(core.view().storage_busy);
    finish(&core, &mut effects, StorageResult::ImportCancelled);
    let after = core.view();
    assert!(!after.storage_busy);
    assert!(after.document_error.is_none() && after.import_summary.is_none());
    assert_eq!(after.documents, before.documents);
    assert_eq!(after.reader, before.reader);
    assert_eq!(after.destination, before.destination);
    assert_eq!(after.content_route, before.content_route);
}

#[test]
fn typography_list_load_and_late_save_do_not_restore_closed_reader() {
    use yomoku_core::typography::{ReaderFontSize, ReaderTypography};
    let core = Core::<Yomoku>::new();
    let typography = ReaderTypography {
        font_size: ReaderFontSize::Sp20,
        furigana: true,
        ..Default::default()
    };
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        &core,
        &mut effects,
        StorageResult::Listed(vec![], PageLength::default(), typography),
    );
    assert_eq!(core.view().reader_typography, Some(typography));
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetReaderTypography(ReaderTypography::default()));
    core.process_event(Event::CloseSettings);
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTypographySaved(ReaderTypography::default()),
    );
    assert!(!core.view().settings_open);
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader.is_none());
}

#[test]
fn paste_success_returns_to_library_without_opening_reader_and_failures_keep_form() {
    let core = Core::<Yomoku>::new();
    core.process_event(Event::SelectDestination(Destination::Statistics));
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
        StorageResult::Created(document("題𠮷").library_summary().unwrap()),
    );
    let view = core.view();
    assert_eq!(view.content_route, ContentRoute::Main);
    assert_eq!(view.documents.len(), 1);
    assert_eq!(view.documents[0].title, "題𠮷");
    assert!(view.reader.is_none());
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
        StorageResult::Existing(document.clone().library_summary().unwrap()),
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
    core.process_event(Event::OpenSettings);
    core.process_event(Event::CloseSettings);
    let mut effects = core.process_event(Event::RefreshLibrary);
    finish(
        &core,
        &mut effects,
        StorageResult::Listed(
            vec![document.library_summary().unwrap()],
            PageLength::default(),
            Default::default(),
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
fn typography_requires_loaded_settings_commits_before_publish_and_preserves_reader() {
    use yomoku_core::typography::{ReaderFontSize, ReaderLineSpacing, ReaderTypography};
    let core = Core::<Yomoku>::new();
    let typography = ReaderTypography {
        font_size: ReaderFontSize::Sp24,
        line_spacing: ReaderLineSpacing::Percent200,
        furigana: true,
    };
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    assert_eq!(core.view().reader_typography, None);
    core.process_event(Event::CloseSettings);
    let document = document(&"𠮷猫。\n".repeat(300));
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    let previous = core.view();
    assert_eq!(
        previous.reader_typography,
        Some(ReaderTypography::default())
    );
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetReaderTypography(typography));
    assert!(effects.iter().any(|request| matches!(request, Effect::Storage(request) if request.operation == StorageOperation::SaveReaderTypography(typography))));
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    assert_eq!(core.view().reader_typography, previous.reader_typography);
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().reader_typography, previous.reader_typography);
    let mut effects = core.process_event(Event::SetReaderTypography(typography));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTypographySaved(typography),
    );
    assert_eq!(core.view().reader_typography, Some(typography));
    assert_eq!(core.view().reader, previous.reader);
    assert_eq!(core.view().documents, previous.documents);
    assert_eq!(core.view().reader_analysis, previous.reader_analysis);
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
}

#[test]
fn txt_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::imports::{TextEncoding, TxtImport};
    let input = TxtImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.txt".into(),
        encoding: Some(TextEncoding::Utf8),
    };
    let core = Core::<Yomoku>::new();
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    let mut effects = core.process_event(Event::ImportTxt(input.clone()));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::ImportTxt(input.clone())
    );
    assert!(core.view().import_summary.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::TxtEncodingRequired),
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert!(core.view().import_summary.is_none());
    let mut effects = core.process_event(Event::ImportTxt(input));
    let mut imported = document("猫");
    imported.title = "book.txt".into();
    finish(
        &core,
        &mut effects,
        StorageResult::Created(imported.library_summary().unwrap()),
    );
    assert_eq!(core.view().destination, Destination::Library);
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert_eq!(core.view().import_summary.unwrap().title, "book.txt");
    assert!(core.view().reader.is_none());
}

#[test]
fn srt_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::imports::{SrtImport, TextEncoding};
    let input = SrtImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.srt".into(),
        encoding: Some(TextEncoding::Utf8),
    };
    let core = Core::<Yomoku>::new();
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    let mut effects = core.process_event(Event::ImportSrt(input.clone()));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::ImportSrt(input.clone())
    );
    assert!(core.view().import_summary.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::SrtEncodingRequired),
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert!(core.view().import_summary.is_none());
    let mut effects = core.process_event(Event::ImportSrt(input));
    let mut imported = document("猫");
    imported.title = "book.srt".into();
    finish(
        &core,
        &mut effects,
        StorageResult::Created(imported.library_summary().unwrap()),
    );
    assert_eq!(core.view().destination, Destination::Library);
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert_eq!(core.view().import_summary.unwrap().title, "book.srt");
    assert!(core.view().reader.is_none());
}

#[test]
fn epub_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::imports::EpubImport;
    let input = EpubImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.epub".into(),
    };
    let core = Core::<Yomoku>::new();
    core.process_event(Event::SelectDestination(Destination::Vocabulary));
    let mut effects = core.process_event(Event::ImportEpub(input.clone()));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::ImportEpub(input.clone())
    );
    assert!(core.view().import_summary.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::InvalidEpub),
    );
    assert_eq!(core.view().destination, Destination::Vocabulary);
    assert!(core.view().import_summary.is_none());
    let mut effects = core.process_event(Event::ImportEpub(input));
    let mut imported = document("猫");
    imported.title = "book.epub".into();
    finish(
        &core,
        &mut effects,
        StorageResult::Created(imported.library_summary().unwrap()),
    );
    assert_eq!(core.view().destination, Destination::Library);
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert_eq!(core.view().import_summary.unwrap().title, "book.epub");
    assert!(core.view().reader.is_none());
}

#[test]
fn ordered_snapshot_rejects_missing_duplicate_and_foreign_chapter_analysis() {
    use yomoku_core::document::ReaderDocument;
    let mut document: ReaderDocument = document("猫").into();
    let mut second = document.chapters[0].clone();
    second.chapter_id = ChapterId::try_from(4).unwrap();
    document.chapters.push(second);
    let entries = document
        .chapters
        .iter()
        .map(|chapter| ReaderChapterAnalysis {
            chapter_id: chapter.chapter_id,
            analysis: ReaderAnalysis::Missing,
        })
        .collect::<Vec<_>>();
    for analyses in [
        vec![],
        vec![entries[0]],
        vec![entries[0], entries[0]],
        vec![
            entries[0],
            ReaderChapterAnalysis {
                chapter_id: ChapterId::try_from(99).unwrap(),
                ..entries[1]
            },
        ],
    ] {
        let core = Core::<Yomoku>::new();
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        finish(
            &core,
            &mut effects,
            StorageResult::Loaded(DocumentReaderSnapshot {
                last_read_sequence: None,
                typography: Default::default(),
                document: document.clone(),
                page_length: PageLength::default(),
                analyses,
            }),
        );
        assert_eq!(
            core.view().document_error,
            Some(DocumentError::InvalidDocument)
        );
        assert!(core.view().reader.is_none());
    }
    // Key order can differ from source order. Missing stays explicit per chapter.
    let core = Core::<Yomoku>::new();
    document.reading_position = Some(ReadingPosition {
        source_id: document.chapters[1].source_id,
        chapter_id: document.chapters[1].chapter_id,
        offset: TextOffset::START,
    });
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        StorageResult::Loaded(DocumentReaderSnapshot {
            last_read_sequence: None,
            typography: Default::default(),
            document: document.clone(),
            page_length: PageLength::default(),
            analyses: vec![
                entries[1],
                ReaderChapterAnalysis {
                    analysis: ReaderAnalysis::Available(AnalysisId::try_from(42).unwrap()),
                    ..entries[0]
                },
            ],
        }),
    );
    assert_eq!(core.view().reader_analysis, Some(ReaderAnalysis::Missing));
    assert_eq!(core.view().reader.unwrap().page_number, 2);
}
