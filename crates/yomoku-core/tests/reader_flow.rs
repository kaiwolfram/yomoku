use crux_core::Core;
use yomoku_core::{
    active_time::{ReaderClockAction, ReaderClockSample, SavedReadingTime},
    app::{ContentRoute, Destination, Effect, Event, ImportOutcome, ImportSummary, Yomoku},
    document::{
        AnalysisId, ChapterId, ChapterText, DocumentId, PastedDocument, SourceId, TextOffset,
    },
    pagination::{PageLength, ReadingPosition},
    reader::{
        progress_percent, DocumentReaderSnapshot, ReaderAnalysis, ReaderChapterAnalysis,
        ReaderPage, ReaderStatusChange, ReaderToken, ReaderTokenKey, ReaderTokenSnapshot,
        ReaderVocabulary,
    },
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::VocabularyStatus,
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
                StorageResult::Listed(
                    core.view().documents,
                    PageLength::default(),
                    Default::default(),
                )
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
        StorageResult::PositionSaved(document.clone().into()),
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
            document.clone().into(),
            yomoku_core::completion::AnalyzedCompletionOutcome::AlreadyRecorded,
        ),
    );
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert!(core.view().reader_time.is_none());
    document.reading_position = None;
    document.finished_at_least_once = true;
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::Characters250),
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
        StorageResult::Created(document("題𠮷").library_summary().unwrap()),
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
                snapshot(document.clone(), PageLength::default(), analysis),
            );
            assert_eq!(core.view().reader_analysis, Some(analysis));
            let mut effects = core.process_event(Event::SaveReadingPosition(
                core.view().reader.unwrap().position,
            ));
            finish(
                &core,
                &mut effects,
                StorageResult::PositionSaved(document.clone().into()),
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

#[test]
fn txt_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        imports::{TextEncoding, TxtImport},
    };
    let input = TxtImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.txt".into(),
        encoding: Some(TextEncoding::Utf8),
    };
    for completion in [false, true] {
        let (core, _) = analyzed_reader();
        let before = core.view().reader;
        let saved = restore_time(&core, 10);
        let mut effects = if completion {
            core.process_event(forward(&core, 1234))
        } else {
            core.process_event(Event::CheckpointReadingTime(
                yomoku_core::active_time::ReadingTimeUpdate {
                    pending: PendingReadingTime::from_millis(100).unwrap(),
                    ..saved.value
                },
            ))
        };
        assert!(core
            .process_event(Event::ImportTxt(input.clone()))
            .is_empty());
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(core
            .process_event(Event::ImportTxt(input.clone()))
            .is_empty());
        assert_eq!(core.view().reader, before);
    }
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
    assert!(core.view().reader.is_none() && core.view().loaded_document.is_none());
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

fn token_key(core: &Core<Yomoku>) -> ReaderTokenKey {
    let view = core.view();
    ReaderTokenKey {
        document_id: view.reader.as_ref().unwrap().document_id,
        page: view.reader.unwrap().anchor,
        analysis: view.reader_analysis.unwrap(),
    }
}

fn token_snapshot(key: ReaderTokenKey) -> ReaderTokenSnapshot {
    ReaderTokenSnapshot {
        document_id: key.document_id,
        page: key.page,
        analysis: key.analysis,
        tokens: vec![],
    }
}

fn load_status_tokens(core: &Core<Yomoku>) -> ReaderStatusChange {
    let key = token_key(core);
    let change = ReaderStatusChange {
        key,
        vocabulary_id: 7.try_into().unwrap(),
        status: VocabularyStatus::Learning,
        occurred_at_unix_ms: 1234,
    };
    let mut snapshot = token_snapshot(key);
    // Repeated occurrences share one ID; a different spelling stays independent.
    snapshot.tokens = (0..3)
        .map(|index| {
            let mut anchor = key.page;
            anchor.range.start = (index * 4).try_into().unwrap();
            anchor.range.end = (index * 4 + if index == 2 { 8 } else { 4 })
                .try_into()
                .unwrap();
            ReaderToken {
                token_index: index as u64,
                anchor,
                display_range: anchor.range,
                surface: if index == 2 { "𠮷𠮷" } else { "𠮷" }.into(),
                reading: Some("ヨシ".into()),
                dictionary_form: Some("𠮷".into()),
                vocabulary: Some(ReaderVocabulary {
                    id: if index == 2 {
                        8.try_into().unwrap()
                    } else {
                        change.vocabulary_id
                    },
                    status: VocabularyStatus::New,
                }),
            }
        })
        .collect();
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    finish(
        core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(snapshot),
    );
    change
}

fn status_result(change: ReaderStatusChange) -> StorageResult {
    StorageResult::ReaderStatusChanged(
        change,
        Some(ReaderVocabulary {
            id: change.vocabulary_id,
            status: change.status,
        }),
    )
}

#[test]
fn manual_status_choices_require_loaded_current_vocabulary_and_valid_timestamp() {
    let (core, _) = analyzed_reader();
    let key = token_key(&core);
    let change = ReaderStatusChange {
        key,
        vocabulary_id: 7.try_into().unwrap(),
        status: VocabularyStatus::Learning,
        occurred_at_unix_ms: 0,
    };
    assert!(core
        .process_event(Event::SetReaderStatus(change))
        .is_empty());
    let change = load_status_tokens(&core);
    for invalid in [
        ReaderStatusChange {
            vocabulary_id: 99.try_into().unwrap(),
            ..change
        },
        ReaderStatusChange {
            occurred_at_unix_ms: -1,
            ..change
        },
        ReaderStatusChange {
            key: ReaderTokenKey {
                document_id: 99.try_into().unwrap(),
                ..key
            },
            ..change
        },
        ReaderStatusChange {
            key: ReaderTokenKey {
                analysis: ReaderAnalysis::Missing,
                ..key
            },
            ..change
        },
        ReaderStatusChange {
            key: ReaderTokenKey {
                page: yomoku_core::document::TextAnchor {
                    chapter_id: 99.try_into().unwrap(),
                    ..key.page
                },
                ..key
            },
            ..change
        },
    ] {
        assert!(core
            .process_event(Event::SetReaderStatus(invalid))
            .is_empty());
    }
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::SetReaderStatus(change))
        .is_empty());
    core.process_event(Event::CloseSettings);
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::SetReaderStatus(change)
    );
    assert!(core
        .process_event(Event::SetReaderStatus(change))
        .is_empty());
    finish(&core, &mut effects, status_result(change));
}

#[test]
fn manual_status_results_update_every_visible_occurrence_without_reader_side_effects() {
    let (core, _) = analyzed_reader();
    let change = load_status_tokens(&core);
    restore_time(&core, 123);
    for status in [
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ] {
        let change = ReaderStatusChange { status, ..change };
        let before = core.view();
        let mut expected = before.reader_tokens.clone().unwrap();
        for token in expected.tokens.iter_mut().take(2) {
            token.vocabulary.as_mut().unwrap().status = status;
        }
        let mut effects = core.process_event(Event::SetReaderStatus(change));
        assert_eq!(core.view().reader_tokens, before.reader_tokens);
        finish(&core, &mut effects, status_result(change));
        let after = core.view();
        assert_eq!(after.reader_tokens, Some(expected));
        assert_eq!(after.reader, before.reader);
        assert_eq!(after.loaded_document, before.loaded_document);
        assert_eq!(after.documents, before.documents);
        assert_eq!(after.reader_time, before.reader_time);
        assert_eq!(after.completion_notice, before.completion_notice);
        assert_eq!(after.book_completion_notice, before.book_completion_notice);
    }
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderStatusChanged(change, None),
    );
    let tokens = core.view().reader_tokens.unwrap().tokens;
    assert!(tokens[..2].iter().all(|token| token.vocabulary.is_none()));
    assert!(tokens[2].vocabulary.is_some());
    assert!(core
        .process_event(Event::SetReaderStatus(change))
        .is_empty());
}

#[test]
fn cancelled_or_mismatched_manual_status_callbacks_cannot_attach_or_surface_errors() {
    for exit in [
        Event::CloseDocument,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Nostr),
    ] {
        for failed in [false, true] {
            let (core, _) = analyzed_reader();
            let change = load_status_tokens(&core);
            let mut effects = core.process_event(Event::SetReaderStatus(change));
            core.process_event(exit.clone());
            finish(
                &core,
                &mut effects,
                if failed {
                    StorageResult::Failed(DocumentError::Unavailable)
                } else {
                    status_result(change)
                },
            );
            assert!(core.view().reader_tokens.is_none());
            assert!(core.view().document_error.is_none());
            assert!(!core.view().storage_busy);
            assert!(core
                .process_event(Event::SetReaderStatus(change))
                .is_empty());
        }
    }
    for wrong_entry in [false, true] {
        let (core, _) = analyzed_reader();
        let change = load_status_tokens(&core);
        let before = core.view().reader_tokens;
        let mut effects = core.process_event(Event::SetReaderStatus(change));
        let result = if wrong_entry {
            StorageResult::ReaderStatusChanged(
                change,
                Some(ReaderVocabulary {
                    id: 99.try_into().unwrap(),
                    status: change.status,
                }),
            )
        } else {
            status_result(ReaderStatusChange {
                occurred_at_unix_ms: 999,
                ..change
            })
        };
        finish(&core, &mut effects, result);
        assert_eq!(core.view().reader_tokens, before);
    }
}

#[test]
fn manual_status_failures_retry_same_choice_and_always_drain_final_clock() {
    for failed in [false, true] {
        let (core, _) = analyzed_reader();
        let change = load_status_tokens(&core);
        restore_time(&core, 10);
        core.process_event(clock(&core, 0, ReaderClockAction::Resume));
        let mut effects = core.process_event(Event::SetReaderStatus(change));
        core.process_event(clock(&core, 500, ReaderClockAction::Pause));
        core.process_event(Event::CloseDocument);
        let mut effects = resolve(
            &core,
            &mut effects,
            if failed {
                StorageResult::Failed(DocumentError::Unavailable)
            } else {
                status_result(change)
            },
        );
        save_clock(&core, &mut effects, 510);
        assert_eq!(core.view().content_route, ContentRoute::Main);
        assert!(!core.view().reading_time_save_pending);
    }
    let (core, _) = analyzed_reader();
    let change = load_status_tokens(&core);
    let before = core.view().reader_tokens;
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().reader_tokens, before);
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    finish(&core, &mut effects, status_result(change));
    assert!(core.view().document_error.is_none());
}

#[test]
fn token_loads_require_current_identity_and_reject_duplicate_or_stale_callbacks() {
    let (core, _) = analyzed_reader();
    let key = token_key(&core);
    let invalid = [
        ReaderTokenKey {
            document_id: 99.try_into().unwrap(),
            ..key
        },
        ReaderTokenKey {
            page: yomoku_core::document::TextAnchor {
                chapter_id: 99.try_into().unwrap(),
                ..key.page
            },
            ..key
        },
        ReaderTokenKey {
            page: yomoku_core::document::TextAnchor {
                range: yomoku_core::document::TextRange {
                    end: key.page.range.start,
                    ..key.page.range
                },
                ..key.page
            },
            ..key
        },
        ReaderTokenKey {
            analysis: ReaderAnalysis::Missing,
            ..key
        },
        ReaderTokenKey {
            analysis: ReaderAnalysis::Available(99.try_into().unwrap()),
            ..key
        },
    ];
    for bad in invalid {
        assert!(core.process_event(Event::LoadReaderTokens(bad)).is_empty());
        // Even a mismatched result on an accepted request must be discarded.
        let mut effects = core.process_event(Event::LoadReaderTokens(key));
        assert!(core.process_event(Event::LoadReaderTokens(key)).is_empty());
        finish(
            &core,
            &mut effects,
            StorageResult::ReaderTokensLoaded(token_snapshot(bad)),
        );
        assert!(core.view().reader_tokens.is_none());
    }
    core.process_event(Event::OpenSettings);
    assert!(core.process_event(Event::LoadReaderTokens(key)).is_empty());
    core.process_event(Event::CloseSettings);
    let before = core.view();
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::LoadReaderTokens(key)
    );
    let snapshot = token_snapshot(key);
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(snapshot.clone()),
    );
    let after = core.view();
    assert_eq!(after.reader_tokens, Some(snapshot));
    assert_eq!(after.reader, before.reader);
    assert_eq!(after.loaded_document, before.loaded_document);
    assert_eq!(after.reader_time, before.reader_time);
    assert_eq!(after.completion_notice, before.completion_notice);
    core.process_event(Event::CloseDocument);
    assert!(core.view().reader_tokens.is_none());
    assert!(core.process_event(Event::LoadReaderTokens(key)).is_empty());
}

#[test]
fn token_reads_clear_on_reflow_movement_and_navigation_and_ignore_cancelled_errors() {
    for exit in [
        Event::CloseDocument,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Nostr),
    ] {
        for failed in [false, true] {
            let (core, _) = analyzed_reader();
            let key = token_key(&core);
            let mut effects = core.process_event(Event::LoadReaderTokens(key));
            core.process_event(exit.clone());
            let result = if failed {
                StorageResult::Failed(DocumentError::Unavailable)
            } else {
                StorageResult::ReaderTokensLoaded(token_snapshot(key))
            };
            finish(&core, &mut effects, result);
            assert!(core.view().reader_tokens.is_none());
            assert!(core.view().document_error.is_none());
            assert!(!core.view().storage_busy);
        }
    }
    let (core, mut document) = analyzed_reader();
    let original = token_key(&core);
    let mut effects = core.process_event(Event::LoadReaderTokens(original));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(token_snapshot(original)),
    );
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters1500));
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters1500),
    );
    assert!(core.view().reader_tokens.is_none());
    core.process_event(Event::CloseSettings);
    assert!(core
        .process_event(Event::LoadReaderTokens(original))
        .is_empty());
    let key = token_key(&core);
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(token_snapshot(key)),
    );
    let position = ReadingPosition {
        offset: key.page.range.end,
        ..core.view().reader.unwrap().position
    };
    let mut effects = core.process_event(Event::SaveReadingPosition(position));
    document.reading_position = Some(position);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.into()),
    );
    assert!(core.view().reader_tokens.is_none());
}

#[test]
fn token_read_failure_is_retryable_and_does_not_block_final_time_or_departure() {
    for failed in [false, true] {
        let (core, _) = analyzed_reader();
        let saved = restore_time(&core, 10);
        core.process_event(clock(&core, 0, ReaderClockAction::Resume));
        let key = token_key(&core);
        let mut effects = core.process_event(Event::LoadReaderTokens(key));
        core.process_event(clock(&core, 500, ReaderClockAction::Pause));
        core.process_event(Event::CloseDocument);
        assert!(core.view().reading_time_save_pending);
        assert_eq!(core.view().reader_time, Some(saved.value));
        let result = if failed {
            StorageResult::Failed(DocumentError::Unavailable)
        } else {
            StorageResult::ReaderTokensLoaded(token_snapshot(key))
        };
        let mut effects = resolve(&core, &mut effects, result);
        save_clock(&core, &mut effects, 510);
        assert_eq!(core.view().content_route, ContentRoute::Main);
        assert!(core.view().reader_tokens.is_none());
        assert!(!core.view().reading_time_save_pending);
    }
    let (core, _) = analyzed_reader();
    let key = token_key(&core);
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().document_error, Some(DocumentError::Unavailable));
    assert!(!core.view().storage_busy);
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(token_snapshot(key)),
    );
    assert!(core.view().document_error.is_none());
    assert!(core.view().reader_tokens.is_some());
}

#[test]
fn legacy_token_reads_expose_explicit_missing_without_requesting_analysis() {
    let core = Core::<Yomoku>::new();
    let document = document("猫𠮷");
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, loaded(document, PageLength::default()));
    let key = token_key(&core);
    assert_eq!(key.analysis, ReaderAnalysis::Missing);
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(token_snapshot(key)),
    );
    assert_eq!(core.view().reader_tokens, Some(token_snapshot(key)));
    assert!(core.view().completion_notice.is_none());
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
            document.into(),
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
            StorageResult::PageCompleted(document.clone().into(), outcome),
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
        finish(
            &core,
            &mut effects,
            StorageResult::PositionSaved(document.into()),
        );
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

#[test]
fn finished_snapshots_keep_exact_position_until_explicit_reader_entry_and_reflow_keeps_marker() {
    let core = Core::<Yomoku>::new();
    let mut document = document(&format!("{}\n終𠮷", "猫".repeat(500)));
    let id = document.document_id;
    let mut effects = core.process_event(Event::CreatePaste(document.chapter.text.clone()));
    finish(
        &core,
        &mut effects,
        StorageResult::Created(document.clone().library_summary().unwrap()),
    );
    assert!(!core.view().documents[0].finished_at_least_once);
    document.finished_at_least_once = true;
    let eof = ReadingPosition {
        source_id: document.chapter.source_id,
        chapter_id: document.chapter.chapter_id,
        offset: TextOffset::at(&document.chapter.text, document.chapter.text.len()).unwrap(),
    };
    document.reading_position = Some(eof);
    let mut effects = core.process_event(Event::LoadPaste(id));
    assert_eq!(storage_operation(&effects), StorageOperation::LoadPaste(id));
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
    assert!(core.view().reader.is_none());
    assert_eq!(core.view().loaded_document, Some(document.clone().into()));
    assert!(core.view().documents[0].finished_at_least_once);
    assert_eq!(core.view().documents[0].progress_percent, 100);
    let mut effects = core.process_event(Event::OpenReader(id));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::OpenReader(id)
    );
    assert!(core.process_event(Event::OpenReader(id)).is_empty());
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().documents[0].progress_percent, 100);
    core.process_event(Event::CloseDocument);
    let mut effects = core.process_event(Event::OpenReader(id));
    document.reading_position = Some(ReadingPosition {
        offset: TextOffset::START,
        ..eof
    });
    finish(
        &core,
        &mut effects,
        loaded(document.clone(), PageLength::default()),
    );
    assert_eq!(core.view().reader.as_ref().unwrap().page_number, 1);
    assert!(core.view().reader.as_ref().unwrap().finished_at_least_once);
    assert_eq!(core.view().documents[0].progress_percent, 0);
    assert!(core.view().documents[0].finished_at_least_once);

    // A completion reload must keep EOF even though the document is finished.
    let mut effects = core.process_event(Event::SaveReadingPosition(eof));
    document.reading_position = Some(eof);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.into()),
    );
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters250),
    );
    let page = core.view().reader.unwrap();
    assert_eq!(page.position, eof);
    assert!(page.finished_at_least_once);
    assert_eq!(core.view().documents[0].progress_percent, 100);
    assert!(core.view().documents[0].finished_at_least_once);
}

#[test]
fn final_page_returns_to_library_and_celebrates_each_recorded_command_even_zero_and_rereads() {
    use yomoku_core::{completion::AnalyzedCompletionOutcome, document::CompletionCommandId};
    for outcome in [
        AnalyzedCompletionOutcome::Recorded { newly_known: 3 },
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 },
        AnalyzedCompletionOutcome::AlreadyRecorded,
    ] {
        let (core, mut document) = analyzed_reader();
        for command in [99, 100] {
            if command == 100 {
                // A finished-book reopen supplies page 1 while retaining the marker.
                document.reading_position = None;
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
            }
            let pages = yomoku_core::pagination::LogicalPages::new(
                std::slice::from_ref(&document.chapter),
                PageLength::default(),
            )
            .unwrap();
            let last = *pages.pages().last().unwrap();
            let position = ReadingPosition {
                source_id: last.source_id,
                chapter_id: last.chapter_id,
                offset: last.range.start,
            };
            let mut effects = core.process_event(Event::SaveReadingPosition(position));
            document.reading_position = Some(position);
            finish(
                &core,
                &mut effects,
                StorageResult::PositionSaved(document.clone().into()),
            );
            let request = forward(&core, 1234);
            let mut effects = core.process_event(request.clone());
            assert!(core.view().book_completion_notice.is_none());
            let mut payload = prepared(storage_operation(&effects));
            payload.completion.command_id = CompletionCommandId::try_from(command).unwrap();
            let mut effects = resolve(
                &core,
                &mut effects,
                StorageResult::PageCompletionPrepared(payload),
            );
            finish(
                &core,
                &mut effects,
                StorageResult::Failed(DocumentError::Unavailable),
            );
            assert_eq!(core.view().content_route, ContentRoute::Reader);
            assert!(core.view().book_completion_notice.is_none());
            let mut effects = core.process_event(Event::RetryReaderCompletion);
            assert_eq!(
                storage_operation(&effects),
                StorageOperation::CompletePage(payload)
            );
            document.reading_position = Some(payload.completion.next_position(&document).unwrap());
            document.finished_at_least_once = true;
            finish(
                &core,
                &mut effects,
                StorageResult::PageCompleted(document.clone().into(), outcome),
            );
            let view = core.view();
            assert_eq!(view.content_route, ContentRoute::Main);
            assert_eq!(view.destination, Destination::Library);
            assert!(
                view.reader.is_none()
                    && view.reader_time.is_none()
                    && view.reader_analysis.is_none()
            );
            assert!(view.documents[0].finished_at_least_once);
            assert_eq!(view.documents[0].progress_percent, 100);
            assert_eq!(
                view.book_completion_notice,
                match outcome {
                    AnalyzedCompletionOutcome::Recorded { .. } =>
                        Some(payload.completion.command_id),
                    AnalyzedCompletionOutcome::AlreadyRecorded => None,
                }
            );
            assert_eq!(
                view.completion_notice.map(|notice| notice.newly_known),
                match outcome {
                    AnalyzedCompletionOutcome::Recorded { newly_known } if newly_known > 0 =>
                        Some(newly_known),
                    _ => None,
                }
            );
            assert!(core.process_event(request).is_empty());
            assert!(core.process_event(Event::RetryReaderCompletion).is_empty());
            assert_eq!(core.view(), view); // Duplicate callbacks cannot replay or change the notice.
            core.process_event(Event::DismissBookCompletionNotice(
                CompletionCommandId::try_from(1).unwrap(),
            ));
            assert_eq!(
                core.view().book_completion_notice,
                view.book_completion_notice
            );
            core.process_event(Event::DismissBookCompletionNotice(
                payload.completion.command_id,
            ));
            assert!(core.view().book_completion_notice.is_none());
        }
    }
}

#[test]
fn finish_book_waits_for_final_clock_checkpoint_and_preserves_celebration_after_deferred_exit() {
    use yomoku_core::completion::AnalyzedCompletionOutcome;
    let (core, mut document) = analyzed_reader();
    let pages = yomoku_core::pagination::LogicalPages::new(
        std::slice::from_ref(&document.chapter),
        PageLength::default(),
    )
    .unwrap();
    let last = *pages.pages().last().unwrap();
    let position = ReadingPosition {
        source_id: last.source_id,
        chapter_id: last.chapter_id,
        offset: last.range.start,
    };
    let mut effects = core.process_event(Event::SaveReadingPosition(position));
    document.reading_position = Some(position);
    finish(
        &core,
        &mut effects,
        StorageResult::PositionSaved(document.clone().into()),
    );
    restore_time(&core, 10);
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 500, ReaderClockAction::Pause));
    core.process_event(forward(&core, 1234));
    core.process_event(Event::CloseDocument);
    assert_eq!(core.view().content_route, ContentRoute::Reader);
    assert!(core.view().book_completion_notice.is_none());
    let mut effects = save_clock(&core, &mut effects, 510);
    let payload = prepared(storage_operation(&effects));
    let mut effects = resolve(
        &core,
        &mut effects,
        StorageResult::PageCompletionPrepared(payload),
    );
    document.reading_position = Some(payload.completion.next_position(&document).unwrap());
    document.finished_at_least_once = true;
    finish(
        &core,
        &mut effects,
        StorageResult::PageCompleted(
            document.into(),
            AnalyzedCompletionOutcome::Recorded { newly_known: 0 },
        ),
    );
    assert_eq!(core.view().content_route, ContentRoute::Main);
    assert_eq!(
        core.view().book_completion_notice,
        Some(payload.completion.command_id)
    );
    assert!(core.view().reader_time.is_none() && !core.view().reading_time_save_pending);
}

fn word_details(core: &Core<Yomoku>, token_index: u64) -> yomoku_core::reader::ReaderWordDetails {
    let token = core
        .view()
        .reader_tokens
        .unwrap()
        .tokens
        .into_iter()
        .find(|t| t.token_index == token_index)
        .unwrap();
    yomoku_core::reader::ReaderWordDetails {
        key: token_key(core),
        encountered: 17,
        candidates: yomoku_core::dictionary::DictionaryCandidates {
            surface: token.surface.clone(),
            dictionary_form: token.dictionary_form.clone(),
            words: vec![],
            names: vec![],
        },
        token,
    }
}

#[test]
fn selected_word_requires_visible_token_and_preserves_reader_state_with_live_status() {
    use yomoku_core::reader::ReaderWordSelection;
    let (core, _) = analyzed_reader();
    let selection = ReaderWordSelection {
        key: token_key(&core),
        token_index: 0,
    };
    assert!(core
        .process_event(Event::LoadReaderWord(selection))
        .is_empty());
    load_status_tokens(&core);
    for invalid in [
        ReaderWordSelection {
            token_index: 99,
            ..selection
        },
        ReaderWordSelection {
            key: ReaderTokenKey {
                analysis: ReaderAnalysis::Missing,
                ..selection.key
            },
            ..selection
        },
    ] {
        assert!(core
            .process_event(Event::LoadReaderWord(invalid))
            .is_empty());
    }
    core.process_event(Event::OpenSettings);
    assert!(core
        .process_event(Event::LoadReaderWord(selection))
        .is_empty());
    core.process_event(Event::CloseSettings);
    let before = core.view();
    let mut details = word_details(&core, 0);
    details.token.vocabulary.as_mut().unwrap().status = VocabularyStatus::Known;
    let mut effects = core.process_event(Event::LoadReaderWord(selection));
    assert_eq!(
        storage_operation(&effects),
        StorageOperation::LoadReaderWord(selection)
    );
    assert!(core
        .process_event(Event::LoadReaderWord(ReaderWordSelection {
            token_index: 1,
            ..selection
        }))
        .is_empty());
    assert_eq!(core.view().reader_word_selection, Some(selection));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderWordLoaded(Box::new(details.clone())),
    );
    assert_eq!(core.view().reader_word, Some(details));
    let after = core.view();
    assert!(after.reader_word_error.is_none());
    assert_eq!(after.reader, before.reader);
    assert_eq!(after.reader_time, before.reader_time);
    assert_eq!(after.documents, before.documents);
    let tokens = after.reader_tokens.unwrap().tokens;
    assert_eq!(
        tokens[0].vocabulary.unwrap().status,
        VocabularyStatus::Known
    );
    assert_eq!(tokens[1].vocabulary, tokens[0].vocabulary);
    assert_eq!(tokens[2].vocabulary.unwrap().status, VocabularyStatus::New);
}

#[test]
fn selected_word_cancellation_rejects_late_success_and_errors_even_after_returning_to_reader() {
    for exit in [
        Event::DismissReaderWord,
        Event::CloseDocument,
        Event::OpenPaste,
        Event::SelectDestination(Destination::Nostr),
        Event::OpenSettings,
    ] {
        for failed in [false, true] {
            let (core, _) = analyzed_reader();
            load_status_tokens(&core);
            let details = word_details(&core, 0);
            let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
            core.process_event(exit.clone());
            core.process_event(Event::CloseSettings);
            finish(
                &core,
                &mut effects,
                if failed {
                    StorageResult::Failed(DocumentError::Unavailable)
                } else {
                    StorageResult::ReaderWordLoaded(Box::new(details))
                },
            );
            assert!(core.view().reader_word.is_none());
            assert!(core.view().reader_word_selection.is_none());
            assert!(core.view().reader_word_error.is_none());
            assert!(core.view().document_error.is_none());
        }
    }
    assert!(Core::<Yomoku>::new().view().reader_word_selection.is_none());
}

#[test]
fn selected_word_ignores_wrong_selection_or_token_identity_and_clears_on_refresh_movement_reflow() {
    for mismatch in 0..3 {
        let (core, _) = analyzed_reader();
        load_status_tokens(&core);
        let mut details = word_details(&core, 0);
        let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
        match mismatch {
            0 => details.token.token_index = 1,
            1 => details.key.analysis = ReaderAnalysis::Missing,
            _ => details.token.surface = "foreign".into(),
        }
        finish(
            &core,
            &mut effects,
            StorageResult::ReaderWordLoaded(Box::new(details)),
        );
        assert!(core.view().reader_word.is_none());
    }
    for invalidation in 0..4 {
        let (core, _) = analyzed_reader();
        load_status_tokens(&core);
        let details = word_details(&core, 0);
        let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
        finish(
            &core,
            &mut effects,
            StorageResult::ReaderWordLoaded(Box::new(details)),
        );
        assert!(core.view().reader_word.is_some());
        let event = match invalidation {
            0 => Event::LoadReaderTokens(token_key(&core)),
            1 => Event::SaveReadingPosition(core.view().reader.unwrap().position),
            2 => Event::RefreshLibrary,
            _ => Event::OpenSettings,
        };
        core.process_event(event);
        assert!(core.view().reader_word_selection.is_none());
        assert!(core.view().reader_word.is_none());
    }
}

#[test]
fn selected_word_failure_survives_clock_saves_retries_explicitly_and_drains_before_departure() {
    for failed in [false, true] {
        let (core, _) = analyzed_reader();
        load_status_tokens(&core);
        restore_time(&core, 10);
        core.process_event(clock(&core, 0, ReaderClockAction::Resume));
        let details = word_details(&core, 0);
        let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
        core.process_event(clock(&core, 500, ReaderClockAction::Pause));
        core.process_event(Event::CloseDocument);
        let result = if failed {
            StorageResult::Failed(DocumentError::Unavailable)
        } else {
            StorageResult::ReaderWordLoaded(Box::new(details))
        };
        let mut effects = resolve(&core, &mut effects, result);
        save_clock(&core, &mut effects, 510);
        assert_eq!(core.view().content_route, ContentRoute::Main);
        assert!(!core.view().reading_time_save_pending);
    }
    let (core, _) = analyzed_reader();
    load_status_tokens(&core);
    restore_time(&core, 10);
    let details = word_details(&core, 0);
    let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(
        core.view().reader_word_error,
        Some(DocumentError::Unavailable)
    );
    assert!(core.view().document_error.is_none());
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 500, ReaderClockAction::Checkpoint));
    save_clock(&core, &mut effects, 510);
    assert_eq!(
        core.view().reader_word_error,
        Some(DocumentError::Unavailable)
    );
    let mut effects = core.process_event(Event::LoadReaderWord(details.selection()));
    assert!(core.view().reader_word_error.is_none());
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderWordLoaded(Box::new(details.clone())),
    );
    let change = ReaderStatusChange {
        key: details.key,
        vocabulary_id: details.token.vocabulary.unwrap().id,
        status: VocabularyStatus::Ignored,
        occurred_at_unix_ms: 100,
    };
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    finish(&core, &mut effects, status_result(change));
    assert_eq!(
        core.view()
            .reader_word
            .unwrap()
            .token
            .vocabulary
            .unwrap()
            .status,
        VocabularyStatus::Ignored
    );
    let mut effects = core.process_event(Event::SetReaderStatus(change));
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderStatusChanged(change, None),
    );
    assert_eq!(core.view().reader_word.unwrap().encountered, 0);
}

#[test]
fn srt_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::{
        active_time::PendingReadingTime,
        imports::{SrtImport, TextEncoding},
    };
    let input = SrtImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.srt".into(),
        encoding: Some(TextEncoding::Utf8),
    };
    for completion in [false, true] {
        let (core, _) = analyzed_reader();
        let before = core.view().reader;
        let saved = restore_time(&core, 10);
        let mut effects = if completion {
            core.process_event(forward(&core, 1234))
        } else {
            core.process_event(Event::CheckpointReadingTime(
                yomoku_core::active_time::ReadingTimeUpdate {
                    pending: PendingReadingTime::from_millis(100).unwrap(),
                    ..saved.value
                },
            ))
        };
        assert!(core
            .process_event(Event::ImportSrt(input.clone()))
            .is_empty());
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(core
            .process_event(Event::ImportSrt(input.clone()))
            .is_empty());
        assert_eq!(core.view().reader, before);
    }
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
    assert!(core.view().reader.is_none() && core.view().loaded_document.is_none());
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
fn epub_import_waits_for_reader_saves_and_only_committed_success_returns_to_library() {
    use yomoku_core::{active_time::PendingReadingTime, imports::EpubImport};
    let input = EpubImport {
        original_bytes: "猫".as_bytes().to_vec(),
        filename: "book.epub".into(),
    };
    for completion in [false, true] {
        let (core, _) = analyzed_reader();
        let before = core.view().reader;
        let saved = restore_time(&core, 10);
        let mut effects = if completion {
            core.process_event(forward(&core, 1234))
        } else {
            core.process_event(Event::CheckpointReadingTime(
                yomoku_core::active_time::ReadingTimeUpdate {
                    pending: PendingReadingTime::from_millis(100).unwrap(),
                    ..saved.value
                },
            ))
        };
        assert!(core
            .process_event(Event::ImportEpub(input.clone()))
            .is_empty());
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert!(core
            .process_event(Event::ImportEpub(input.clone()))
            .is_empty());
        assert_eq!(core.view().reader, before);
    }
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
    assert!(core.view().reader.is_none() && core.view().loaded_document.is_none());
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
        assert!(core.view().reader.is_none() && core.view().loaded_document.is_none());
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
    assert_eq!(core.view().loaded_document, Some(document));
    core.process_event(forward(&core, 123));
    assert_eq!(
        core.view().document_error,
        Some(DocumentError::MissingAnalysis)
    );
}

#[test]
fn furigana_only_changes_commit_without_replacing_reader_tokens_time_or_notices() {
    use yomoku_core::typography::ReaderTypography;
    let (core, _) = analyzed_reader();
    load_status_tokens(&core);
    restore_time(&core, 321);
    core.process_event(Event::OpenSettings);
    assert!(!core.view().reader_typography.unwrap().furigana);
    for furigana in [true, false] {
        let before = core.view();
        let typography = ReaderTypography {
            furigana,
            ..before.reader_typography.unwrap()
        };
        let mut effects = core.process_event(Event::SetReaderTypography(typography));
        assert_eq!(
            storage_operation(&effects),
            StorageOperation::SaveReaderTypography(typography)
        );
        assert_eq!(core.view().reader_typography, before.reader_typography);
        finish(
            &core,
            &mut effects,
            StorageResult::Failed(DocumentError::Unavailable),
        );
        assert_eq!(core.view().reader_typography, before.reader_typography);
        let mut effects = core.process_event(Event::SetReaderTypography(typography));
        finish(
            &core,
            &mut effects,
            StorageResult::ReaderTypographySaved(typography),
        );
        let after = core.view();
        assert_eq!(
            after,
            yomoku_core::app::ViewModel {
                reader_typography: Some(typography),
                reader_furigana: if furigana {
                    before
                        .reader
                        .as_ref()
                        .unwrap()
                        .furigana(
                            before.reader_analysis.unwrap(),
                            before.reader_tokens.as_ref(),
                        )
                        .unwrap()
                } else {
                    vec![]
                },
                ..before
            }
        );
        assert!(core
            .process_event(Event::SetReaderTypography(typography))
            .is_empty());
    }
}

#[test]
fn furigana_projection_tracks_source_tokens_refresh_reflow_and_departure_without_io() {
    use yomoku_core::reader_ruby::{ReaderChapterRuby, ReaderRuby};
    use yomoku_core::typography::ReaderTypography;
    let core = Core::<Yomoku>::new();
    let document = document(&format!("{}\n{}\n猫。", "𠮷".repeat(250), "𠮷".repeat(250)));
    let StorageResult::Loaded(mut saved) = snapshot(
        document.clone(),
        PageLength::default(),
        ReaderAnalysis::Available(42.try_into().unwrap()),
    ) else {
        unreachable!()
    };
    let source = ReaderRuby {
        range: yomoku_core::document::TextRange {
            start: 4.try_into().unwrap(),
            end: 8.try_into().unwrap(),
        },
        reading: "ソース𠮷\0か\u{3099}".into(),
    };
    saved.document.source_ruby = vec![ReaderChapterRuby {
        chapter_id: document.chapter.chapter_id,
        ruby: vec![source.clone()],
    }];
    let mut effects = core.process_event(Event::OpenReader(document.document_id));
    finish(&core, &mut effects, StorageResult::Loaded(saved));
    assert!(core.view().reader_furigana.is_empty());
    let page = core.view().reader.unwrap();
    core.process_event(Event::OpenSettings);
    let typography = ReaderTypography {
        furigana: true,
        ..Default::default()
    };
    let mut effects = core.process_event(Event::SetReaderTypography(typography));
    assert!(core.view().reader_furigana.is_empty());
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTypographySaved(typography),
    );
    assert_eq!(core.view().reader_furigana, std::slice::from_ref(&source));
    core.process_event(Event::CloseSettings);
    let change = load_status_tokens(&core);
    let ruby = core.view().reader_furigana;
    assert_eq!(ruby.len(), 3);
    assert_eq!(ruby[1], source);
    assert_eq!(ruby[0].reading, "ヨシ");
    assert_eq!(ruby[2].reading, "ヨシ");
    let before = core.view();
    for status in [
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ] {
        let change = ReaderStatusChange { status, ..change };
        let mut effects = core.process_event(Event::SetReaderStatus(change));
        finish(&core, &mut effects, status_result(change));
        assert_eq!(core.view().reader_furigana, ruby);
        assert_eq!(core.view().reader, Some(page.clone()));
    }
    let key = token_key(&core);
    let mut effects = core.process_event(Event::LoadReaderTokens(key));
    assert_eq!(core.view().reader_furigana, std::slice::from_ref(&source));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert_eq!(core.view().reader_furigana, std::slice::from_ref(&source));
    load_status_tokens(&core);
    assert_eq!(core.view().reader_furigana, ruby);
    assert_eq!(core.view().loaded_document, before.loaded_document);
    assert_eq!(core.view().reader_time, before.reader_time);
    assert_eq!(core.view().completion_notice, before.completion_notice);
    core.process_event(Event::OpenSettings);
    let mut effects = core.process_event(Event::SetPageLength(PageLength::Characters250));
    finish(
        &core,
        &mut effects,
        StorageResult::PageLengthSaved(PageLength::Characters250),
    );
    assert_eq!(core.view().reader_furigana, [source]);
    assert_eq!(core.view().reader.unwrap().position, page.position);
    core.process_event(Event::CloseSettings);
    let mut effects = core.process_event(Event::LoadReaderTokens(token_key(&core)));
    core.process_event(Event::CloseDocument);
    finish(
        &core,
        &mut effects,
        StorageResult::ReaderTokensLoaded(token_snapshot(key)),
    );
    assert!(core.view().reader_furigana.is_empty());
    assert!(core.view().reader.is_none());
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
    assert_eq!(core.view().loaded_document, previous.loaded_document);
    assert_eq!(core.view().reader_analysis, previous.reader_analysis);
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
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
fn typography_waits_for_failed_time_and_completion_saves_before_accepting_changes() {
    use yomoku_core::typography::{ReaderFontSize, ReaderTypography};
    let typography = ReaderTypography {
        font_size: ReaderFontSize::Sp24,
        furigana: true,
        ..Default::default()
    };
    let (core, _) = analyzed_reader();
    restore_time(&core, 0);
    core.process_event(clock(&core, 0, ReaderClockAction::Resume));
    let mut effects = core.process_event(clock(&core, 100, ReaderClockAction::Pause));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::OpenSettings);
    assert!(core.view().reading_time_save_pending);
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    let mut effects = core.process_event(Event::RetryReadingTimeCheckpoint);
    save_clock(&core, &mut effects, 100);
    core.process_event(Event::CloseSettings);
    let mut effects = core.process_event(forward(&core, 123));
    finish(
        &core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    core.process_event(Event::OpenSettings);
    assert!(core.view().completion_save_pending);
    assert!(core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    assert_eq!(
        core.view().reader_typography,
        Some(ReaderTypography::default())
    );
}
