use std::sync::Arc;

use crux_core::Core;
use yomoku_core::{
    active_time::{PendingReadingTime, ReaderClockAction, ReaderClockSample, SavedReadingTime},
    analysis::{AnalysisProfile, AnalyzedToken, ChapterAnalysis, SplitMode},
    app::{ContentRoute, Destination, Effect, Event, Model, Yomoku},
    completion::{AnalyzedCompletionOutcome, PageCompletion},
    document::{ChapterText, ReaderDocument, TextAnchor, TextOffset, TextRange},
    effective_reader::{
        EffectivePageCompletion, EffectiveReaderChapter, EffectiveReaderTokenSnapshot,
    },
    effective_session::{EffectivePageIdentity, EffectiveReaderRequest},
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::{PageLength, ReadingPosition},
    reader::{DocumentReaderSnapshot, ReaderAnalysis, ReaderChapterAnalysis, ReaderTokenKey},
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::Surface,
};

#[path = "effective_app/words.rs"]
mod words;

struct Fixture {
    core: Core<Yomoku>,
    document: ReaderDocument,
    original: ChapterAnalysis,
}

impl Fixture {
    fn new(long: bool) -> Self {
        let chapter = ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            text: "猫犬𠮷か\u{3099}\0。\n".repeat(if long { 150 } else { 1 }),
        };
        let mut tokens = Vec::new();
        let mut start = 0;
        while start < chapter.text.len() {
            for surface in ["猫", "犬", "𠮷か\u{3099}\0", "。"] {
                let end = start + surface.len();
                tokens.push(AnalyzedToken {
                    anchor: TextAnchor {
                        source_id: chapter.source_id,
                        chapter_id: chapter.chapter_id,
                        range: range(start, end),
                    },
                    surface: surface.into(),
                    is_japanese: surface != "。",
                    reading: Some(format!("読み{surface}")),
                    dictionary_form: Some(Surface::new(surface).unwrap()),
                });
                start = end;
            }
            start += 1; // newline is canonical whitespace, not a word
        }
        let original = ChapterAnalysis {
            profile: AnalysisProfile {
                engine_revision: "app@1".into(),
                dictionary_revision: "full@1".into(),
            },
            mode: SplitMode::C,
            tokens,
        };
        let document = ReaderDocument {
            document_id: 1.try_into().unwrap(),
            title: "effective application".into(),
            finished_at_least_once: false,
            reading_position: Some(ReadingPosition {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                offset: TextOffset::START,
            }),
            chapters: vec![chapter],
            formatting: vec![],
            images: vec![],
            source_ruby: vec![],
        };
        let core = Core::new_with(Yomoku, Model::default());
        let mut effects = core.process_event(Event::OpenReader(document.document_id));
        resolve(
            &core,
            &mut effects,
            StorageResult::Loaded(DocumentReaderSnapshot {
                document: document.clone(),
                page_length: PageLength::default(),
                typography: yomoku_core::typography::ReaderTypography {
                    furigana: true,
                    ..Default::default()
                },
                analyses: vec![ReaderChapterAnalysis {
                    chapter_id: document.chapters[0].chapter_id,
                    analysis: ReaderAnalysis::Available(4.try_into().unwrap()),
                }],
                last_read_sequence: None,
            }),
        );
        Self {
            core,
            document,
            original,
        }
    }

    fn base(&self) -> ReaderTokenKey {
        let view = self.core.view();
        let page = view.reader.unwrap();
        ReaderTokenKey {
            document_id: page.document_id,
            page: page.anchor,
            analysis: view.reader_analysis.unwrap(),
        }
    }

    fn snapshot(&self) -> EffectiveReaderTokenSnapshot {
        let resolved = self.resolved();
        EffectiveReaderTokenSnapshot {
            document_id: self.document.document_id,
            page: self.base().page,
            tokens: resolved.tokens_on_page(self.base().page).unwrap(),
            segmentation: Some(resolved.into_segmentation()),
        }
    }

    fn resolved(&self) -> EffectiveReaderChapter<'_> {
        EffectiveReaderChapter::resolve(
            &self.document.chapters[0],
            4.try_into().unwrap(),
            &self.original,
        )
        .unwrap()
    }

    fn load(&self) -> EffectivePageIdentity {
        let mut effects = self
            .core
            .process_event(Event::LoadReaderTokens(self.base()));
        let StorageOperation::Effective(Op::LoadTokens(request)) = operation(&effects) else {
            panic!("effective read")
        };
        resolve(
            &self.core,
            &mut effects,
            StorageResult::Effective(Output::TokensLoaded(request, self.snapshot())),
        );
        self.core.view().effective_reader_tokens.unwrap().identity
    }

    fn complete(&self, identity: EffectivePageIdentity) -> Event {
        Event::CompleteEffectiveReaderPage {
            identity,
            position: self.core.view().reader.unwrap().position,
            completed_at_unix_ms: 500,
        }
    }

    fn prepare(
        &self,
        effects: &mut [Effect],
        pending: i64,
    ) -> (Vec<Effect>, Arc<EffectivePageCompletion>) {
        let StorageOperation::Effective(Op::PrepareCompletion { intent, command }) =
            operation(effects)
        else {
            panic!("prepare")
        };
        assert_eq!(intent.completed_at_unix_ms, 500);
        assert_eq!(command.key(), &self.snapshot().key().unwrap());
        let prepared = Arc::new(
            self.resolved()
                .freeze_completion(PageCompletion {
                    command_id: 10.try_into().unwrap(),
                    document_id: intent.document_id,
                    page: intent.page,
                    page_length: intent.page_length,
                    departing_position: intent.departing_position,
                    completed_at_unix_ms: intent.completed_at_unix_ms,
                    pending_time: PendingReadingTime::from_millis(pending).unwrap(),
                })
                .unwrap(),
        );
        (
            resolve(
                &self.core,
                effects,
                StorageResult::Effective(Output::CompletionPrepared(prepared.clone())),
            ),
            prepared,
        )
    }

    fn finished_document(&self, prepared: &EffectivePageCompletion) -> ReaderDocument {
        let mut document = self.document.clone();
        document.reading_position = Some(
            prepared
                .completion()
                .next_document_position(&document)
                .unwrap(),
        );
        document.finished_at_least_once = document
            .reading_position
            .unwrap()
            .offset
            .byte_index(&document.chapters[0].text)
            .unwrap()
            == document.chapters[0].text.len();
        document
    }
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: (start as i64).try_into().unwrap(),
        end: (end as i64).try_into().unwrap(),
    }
}

fn operation(effects: &[Effect]) -> StorageOperation {
    effects
        .iter()
        .find_map(|effect| {
            if let Effect::Storage(request) = effect {
                Some(request.operation.clone())
            } else {
                None
            }
        })
        .expect("storage effect")
}

fn resolve(core: &Core<Yomoku>, effects: &mut [Effect], result: StorageResult) -> Vec<Effect> {
    let request = effects
        .iter_mut()
        .find_map(|effect| {
            if let Effect::Storage(request) = effect {
                Some(request)
            } else {
                None
            }
        })
        .expect("storage effect");
    core.resolve(request, result).unwrap()
}

fn clock(f: &Fixture, at: i64, action: ReaderClockAction) -> Event {
    let reader = f.core.view().reader.unwrap();
    Event::ObserveReaderClock(ReaderClockSample {
        document_id: reader.document_id,
        position: reader.position,
        monotonic_ms: at,
        action,
    })
}

fn restore(f: &Fixture) {
    let reader = f.core.view().reader.unwrap();
    let mut effects = f.core.process_event(Event::RestoreReadingTime);
    resolve(
        &f.core,
        &mut effects,
        StorageResult::ReadingTimeLoaded(SavedReadingTime {
            value: yomoku_core::active_time::ReadingTimeUpdate {
                document_id: reader.document_id,
                position: reader.position,
                pending: PendingReadingTime::from_millis(0).unwrap(),
            },
            revision: 0,
        }),
    );
}

fn save_time(f: &Fixture, effects: &mut [Effect], expected: i64) -> Vec<Effect> {
    let StorageOperation::SaveReadingTime {
        previous,
        checkpoint,
    } = operation(effects)
    else {
        panic!("checkpoint")
    };
    assert_eq!(checkpoint.pending.milliseconds(), expected);
    resolve(
        &f.core,
        effects,
        StorageResult::ReadingTimeSaved(SavedReadingTime {
            value: checkpoint,
            revision: previous.revision + 1,
        }),
    )
}

#[test]
fn effective_application_loads_compact_page_and_uses_exact_furigana_without_legacy_writes() {
    let f = Fixture::new(false);
    assert!(f.core.view().reader_furigana.is_empty());
    let identity = f.load();
    let view = f.core.view();
    let tokens = view.effective_reader_tokens.unwrap().tokens;
    assert_eq!(
        tokens
            .iter()
            .map(|t| t.surface.as_str())
            .collect::<Vec<_>>(),
        ["猫", "犬", "𠮷か\u{3099}\0"]
    );
    assert!(tokens[0].reading.is_some() && tokens[1].reading.is_some());
    assert_eq!(view.reader_furigana.len(), 3); // exact analyzed word, never borrowed B for 猫/犬
    let _page = view.reader.unwrap();
    let mut effects = f.core.process_event(f.complete(identity));
    let (mut effects, prepared) = f.prepare(&mut effects, 0);
    assert!(f.core.view().completion_save_pending);
    assert!(f.core.view().effective_reader_tokens.is_none());
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::PageCompleted(
            f.finished_document(&prepared),
            AnalyzedCompletionOutcome::Recorded { newly_known: 3 },
        )),
    );
    let view = f.core.view();
    assert_eq!(view.content_route, ContentRoute::Main);
    assert_eq!(view.destination, Destination::Library);
    assert_eq!(view.completion_notice.unwrap().newly_known, 3);
    assert_eq!(
        view.book_completion_notice,
        Some(prepared.completion().command_id)
    );
    assert!(!view.completion_save_pending && !view.storage_busy);
    assert!(f
        .core
        .process_event(f.complete_at_old_position(identity))
        .is_empty());
}

impl Fixture {
    fn complete_at_old_position(&self, identity: EffectivePageIdentity) -> Event {
        Event::CompleteEffectiveReaderPage {
            identity,
            position: self.document.reading_position.unwrap(),
            completed_at_unix_ms: 500,
        }
    }
}

#[test]
fn completion_freezes_identity_then_drains_both_time_slots_and_retries_exactly_before_departure() {
    let f = Fixture::new(true);
    let identity = f.load();
    restore(&f);
    f.core
        .process_event(clock(&f, 0, ReaderClockAction::Resume));
    let mut first = f
        .core
        .process_event(clock(&f, 100, ReaderClockAction::Checkpoint));
    f.core
        .process_event(clock(&f, 250, ReaderClockAction::Pause));
    f.core.process_event(f.complete(identity));
    f.core.process_event(Event::CloseDocument);
    assert!(f.core.view().reading_time_save_pending && f.core.view().completion_save_pending);
    assert!(f.core.view().effective_reader_tokens.is_none());
    assert!(f
        .core
        .process_event(clock(&f, 1000, ReaderClockAction::Resume))
        .is_empty());
    let frozen = operation(&first);
    resolve(
        &f.core,
        &mut first,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    f.core.process_event(Event::OpenSettings);
    assert!(f
        .core
        .process_event(Event::SetPageLength(PageLength::Characters250))
        .is_empty());
    f.core.process_event(Event::CloseSettings);
    assert!(!f.core.view().storage_busy && f.core.view().completion_save_pending);
    let mut retry = f.core.process_event(Event::RetryReadingTimeCheckpoint);
    assert_eq!(operation(&retry), frozen);
    let mut second = save_time(&f, &mut retry, 100);
    let mut preparation = save_time(&f, &mut second, 250);
    let frozen = operation(&preparation);
    resolve(
        &f.core,
        &mut preparation,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(!f.core.view().storage_busy && f.core.view().completion_save_pending);
    assert!(f
        .core
        .process_event(Event::LoadReaderTokens(f.base()))
        .is_empty());
    let mut retry = f.core.process_event(Event::RetryReaderCompletion);
    assert_eq!(operation(&retry), frozen);
    let (mut commit, prepared) = f.prepare(&mut retry, 250);
    let frozen = operation(&commit);
    resolve(
        &f.core,
        &mut commit,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    let mut retry = f.core.process_event(Event::RetryReaderCompletion);
    assert_eq!(operation(&retry), frozen);
    resolve(
        &f.core,
        &mut retry,
        StorageResult::Effective(Output::PageCompleted(
            f.finished_document(&prepared),
            AnalyzedCompletionOutcome::AlreadyRecorded,
        )),
    );
    let view = f.core.view();
    assert_eq!(view.content_route, ContentRoute::Main); // deferred departure waited for exact replay
    assert!(!view.completion_save_pending && !view.reading_time_save_pending && !view.storage_busy);
    assert!(view.completion_notice.is_none() && view.book_completion_notice.is_none());
}

#[test]
fn read_failure_and_dismissal_drain_final_time_and_captured_old_callbacks_cannot_release_new_work()
{
    for dismiss in [false, true] {
        let f = Fixture::new(false);
        restore(&f);
        f.core
            .process_event(clock(&f, 0, ReaderClockAction::Resume));
        let mut read = f.core.process_event(Event::LoadReaderTokens(f.base()));
        let StorageOperation::Effective(Op::LoadTokens(request)) = operation(&read) else {
            panic!("tokens")
        };
        f.core
            .process_event(clock(&f, 200, ReaderClockAction::Pause));
        if dismiss {
            f.core.process_event(Event::CloseDocument);
        }
        // Uncaptured results cannot clear the effective read's FIFO obligation.
        f.core
            .process_event(Event::StorageFinished(StorageResult::Failed(
                DocumentError::Unavailable,
            )));
        assert!(f.core.view().storage_busy);
        let result = if dismiss {
            StorageResult::Effective(Output::TokensLoaded(request, f.snapshot()))
        } else {
            StorageResult::Failed(DocumentError::Unavailable)
        };
        let mut checkpoint = resolve(&f.core, &mut read, result);
        assert!(f.core.view().effective_reader_tokens.is_none());
        f.core.process_event(Event::EffectiveStorageFinished(
            Box::new(EffectiveReaderRequest::Tokens(request)),
            StorageResult::Failed(DocumentError::InvalidDocument),
        ));
        assert!(f.core.view().storage_busy); // checkpoint still owns this slot
        save_time(&f, &mut checkpoint, 200);
        assert!(!f.core.view().storage_busy);
        assert_eq!(
            f.core.view().content_route,
            if dismiss {
                ContentRoute::Main
            } else {
                ContentRoute::Reader
            }
        );
        if !dismiss {
            let identity = f.load();
            assert!(identity.revision > 1);
        }
    }
}

#[test]
fn page_movement_reflow_and_exit_retire_compact_completion_callbacks_before_io() {
    for action in 0..5 {
        let f = Fixture::new(true);
        let identity = f.load();
        match action {
            0 => {
                f.core.process_event(Event::CloseDocument);
            }
            1 => {
                f.core
                    .process_event(Event::SelectDestination(Destination::Vocabulary));
            }
            2 => {
                f.core.process_event(Event::OpenPaste);
            }
            3 => {
                f.core.process_event(Event::SaveReadingPosition(
                    f.core.view().reader.unwrap().position,
                ));
            }
            _ => {
                f.core.process_event(Event::OpenSettings);
                f.core
                    .process_event(Event::SetPageLength(PageLength::Characters250));
            }
        }
        assert!(f.core.view().effective_reader_tokens.is_none());
        assert!(f
            .core
            .process_event(f.complete_at_old_position(identity))
            .is_empty());
    }
}

#[test]
fn refreshed_identity_rejects_stale_completion_and_malformed_token_results_require_explicit_retry()
{
    let f = Fixture::new(false);
    let old = f.load();
    let mut effects = f.core.process_event(Event::LoadReaderTokens(f.base()));
    assert!(f.core.view().effective_reader_tokens.is_none());
    let StorageOperation::Effective(Op::LoadTokens(request)) = operation(&effects) else {
        panic!("tokens")
    };
    let mut malformed = f.snapshot();
    malformed.document_id = 99.try_into().unwrap();
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::TokensLoaded(request, malformed)),
    );
    assert_eq!(
        f.core.view().document_error,
        Some(DocumentError::InvalidDocument)
    );
    assert!(!f.core.view().storage_busy);
    assert!(f.core.process_event(f.complete(old)).is_empty());
    let fresh = f.load();
    assert!(fresh.revision > old.revision);
    assert!(f.core.process_event(f.complete(old)).is_empty());
    f.core.process_event(Event::EffectiveStorageFinished(
        Box::new(EffectiveReaderRequest::Tokens(request)),
        StorageResult::Failed(DocumentError::Unavailable),
    ));
    assert_eq!(
        f.core.view().effective_reader_tokens.unwrap().identity,
        fresh
    );
    assert!(f.core.view().document_error.is_none());
}

#[test]
fn missing_analysis_stays_plain_and_cannot_complete_or_rescan() {
    let f = Fixture::new(false);
    f.core.process_event(Event::CloseDocument);
    let mut effects = f
        .core
        .process_event(Event::OpenReader(f.document.document_id));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Loaded(DocumentReaderSnapshot {
            document: f.document.clone(),
            page_length: PageLength::default(),
            typography: Default::default(),
            analyses: vec![ReaderChapterAnalysis {
                chapter_id: f.document.chapters[0].chapter_id,
                analysis: ReaderAnalysis::Missing,
            }],
            last_read_sequence: None,
        }),
    );
    let mut effects = f.core.process_event(Event::LoadReaderTokens(f.base()));
    let StorageOperation::Effective(Op::LoadTokens(request)) = operation(&effects) else {
        panic!("tokens")
    };
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Effective(Output::TokensLoaded(
            request,
            EffectiveReaderTokenSnapshot {
                document_id: f.document.document_id,
                page: f.base().page,
                segmentation: None,
                tokens: vec![],
            },
        )),
    );
    let page = f.core.view().effective_reader_tokens.unwrap();
    assert!(page.tokens.is_empty());
    assert_eq!(page.identity.base.analysis, ReaderAnalysis::Missing);
    let effects = f.core.process_event(f.complete(page.identity));
    assert!(effects
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
    assert_eq!(
        f.core.view().document_error,
        Some(DocumentError::MissingAnalysis)
    );
    assert!(!f.core.view().completion_save_pending && !f.core.view().storage_busy);
}

#[test]
fn accepted_failed_completion_blocks_typography_and_imports_until_explicit_retry() {
    let f = Fixture::new(true);
    let identity = f.load();
    let mut effects = f.core.process_event(f.complete(identity));
    resolve(
        &f.core,
        &mut effects,
        StorageResult::Failed(DocumentError::Unavailable),
    );
    assert!(f.core.view().completion_save_pending);
    let before = f.core.view().reader;
    for event in [
        Event::CreatePaste("猫".into()),
        Event::ImportTxt(yomoku_core::imports::TxtImport {
            original_bytes: b"text".to_vec(),
            filename: "book.txt".into(),
            encoding: None,
        }),
        Event::ImportSrt(yomoku_core::imports::SrtImport {
            original_bytes: b"text".to_vec(),
            filename: "book.srt".into(),
            encoding: None,
        }),
        Event::ImportEpub(yomoku_core::imports::EpubImport {
            original_bytes: b"text".to_vec(),
            filename: "book.epub".into(),
        }),
    ] {
        assert!(f.core.process_event(event).is_empty());
        assert_eq!(f.core.view().reader, before);
    }
    f.core.process_event(Event::OpenSettings);
    let typography = yomoku_core::typography::ReaderTypography {
        furigana: true,
        ..Default::default()
    };
    assert!(f
        .core
        .process_event(Event::SetReaderTypography(typography))
        .is_empty());
    assert!(f.core.view().completion_save_pending);
    assert!(!f
        .core
        .process_event(Event::RetryReaderCompletion)
        .is_empty());
}
