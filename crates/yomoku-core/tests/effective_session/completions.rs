use std::sync::Arc;

use super::*;
use yomoku_core::{
    completion::{AnalyzedCompletionOutcome, PageCompletion, PageCompletionIntent},
    document::ReaderDocument,
    effective_reader::EffectivePageCompletion,
    effective_session::{EffectiveCompletionProgress as Progress, EffectiveCompletionRequest},
    effective_storage::{EffectiveStorageOperation as Op, EffectiveStorageResult as Output},
    pagination::{PageLength, ReadingPosition},
    storage::DocumentError,
};

struct Fixture {
    document: ReaderDocument,
    original: ChapterAnalysis,
    intent: PageCompletionIntent,
}

impl Fixture {
    fn new() -> Self {
        let chapter = ChapterText {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            text: "猫犬𠮷か\u{3099}\0。".into(),
        };
        let mut start = 0;
        let original = ChapterAnalysis {
            profile: AnalysisProfile {
                engine_revision: "fixture@1".into(),
                dictionary_revision: "full@1".into(),
            },
            mode: SplitMode::C,
            tokens: ["猫", "犬", "𠮷か\u{3099}\0", "。"]
                .into_iter()
                .map(|surface| {
                    let end = start + surface.len();
                    let token = AnalyzedToken {
                        anchor: TextAnchor {
                            source_id: chapter.source_id,
                            chapter_id: chapter.chapter_id,
                            range: range(start, end),
                        },
                        surface: surface.into(),
                        is_japanese: surface != "。",
                        reading: Some(format!("読み{surface}")),
                        dictionary_form: Some(Surface::new(surface).unwrap()),
                    };
                    start = end;
                    token
                })
                .collect(),
        };
        let intent = PageCompletionIntent {
            document_id: 1.try_into().unwrap(),
            page: TextAnchor {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                range: range(0, chapter.text.len()),
            },
            page_length: PageLength::default(),
            departing_position: ReadingPosition {
                source_id: chapter.source_id,
                chapter_id: chapter.chapter_id,
                offset: 0.try_into().unwrap(),
            },
            completed_at_unix_ms: 500,
            analysis_id: 4.try_into().unwrap(),
        };
        Self {
            document: ReaderDocument {
                document_id: intent.document_id,
                title: "fixture".into(),
                finished_at_least_once: false,
                chapters: vec![chapter],
                formatting: vec![],
                images: vec![],
                reading_position: Some(intent.departing_position),
            },
            original,
            intent,
        }
    }

    fn resolved(&self) -> EffectiveReaderChapter<'_> {
        EffectiveReaderChapter::resolve(
            &self.document.chapters[0],
            self.intent.analysis_id,
            &self.original,
        )
        .unwrap()
    }

    fn load(
        &self,
        session: &mut EffectiveReaderSession,
    ) -> yomoku_core::effective_session::EffectivePageIdentity {
        let resolved = self.resolved();
        let snapshot = EffectiveReaderTokenSnapshot {
            document_id: self.intent.document_id,
            page: self.intent.page,
            tokens: resolved.tokens_on_page(self.intent.page).unwrap(),
            segmentation: Some(resolved.into_segmentation()),
        };
        let request = session.begin_load(base(&snapshot)).unwrap();
        assert!(session.accept(request, snapshot));
        session.page().unwrap().identity
    }

    fn prepared(&self, intent: PageCompletionIntent, id: i64) -> Arc<EffectivePageCompletion> {
        Arc::new(
            self.resolved()
                .freeze_completion(PageCompletion {
                    command_id: id.try_into().unwrap(),
                    document_id: intent.document_id,
                    page: intent.page,
                    page_length: intent.page_length,
                    departing_position: intent.departing_position,
                    completed_at_unix_ms: intent.completed_at_unix_ms,
                })
                .unwrap(),
        )
    }

    fn begin(&self, session: &mut EffectiveReaderSession) -> EffectiveCompletionRequest {
        let identity = self.load(session);
        assert!(session.begin_completion(identity, self.intent));
        session.dispatch_completion().unwrap()
    }

    fn prepare(&self, session: &mut EffectiveReaderSession) -> EffectiveCompletionRequest {
        let request = self.begin(session);
        assert_eq!(
            session.finish_completion_request(
                &request,
                Ok(Output::CompletionPrepared(self.prepared(self.intent, 10)))
            ),
            Some(Progress::Prepared)
        );
        session.dispatch_completion().unwrap()
    }

    fn completed_document(&self) -> ReaderDocument {
        let mut document = self.document.clone();
        document.finished_at_least_once = true;
        document.reading_position.as_mut().unwrap().offset = self.intent.page.range.end;
        document
    }
}

#[test]
fn completion_freezes_before_drain_retires_ui_and_rejects_duplicate_forward_actions() {
    let fixture = Fixture::new();
    let mut session = EffectiveReaderSession::default();
    let identity = fixture.load(&mut session);
    let selection = EffectiveReaderSelection {
        page: identity,
        segment_index: 0,
    };
    let key = session.freeze_page(identity).unwrap();
    assert!(session.begin_completion(identity, fixture.intent));
    assert!(session.page().is_none());
    assert!(session.resolve(identity).is_none());
    assert!(session.begin_word_load(selection).is_none());
    assert!(session.completion_state().pending);
    assert!(!session.completion_state().busy);
    assert!(!session.begin_completion(identity, fixture.intent));
    // Accepted provenance survives a page refresh before completion dispatch.
    let refreshed = fixture.load(&mut session);
    assert!(session
        .begin_word_load(EffectiveReaderSelection {
            page: refreshed,
            segment_index: 0
        })
        .is_none());
    assert!(!session.begin_completion(refreshed, fixture.intent));
    let request = session.dispatch_completion().unwrap();
    assert_eq!(
        request.operation(),
        Op::PrepareCompletion {
            intent: fixture.intent,
            command: key
        }
    );
    assert!(session.completion_state().busy);
    assert!(session.dispatch_completion().is_none());
}

#[test]
fn completion_rejects_stale_missing_and_mismatched_intents_without_retiring_the_page() {
    let fixture = Fixture::new();
    let mut session = EffectiveReaderSession::default();
    let old = fixture.load(&mut session);
    let identity = fixture.load(&mut session);
    assert!(!session.begin_completion(old, fixture.intent));
    for field in 0..9 {
        let mut intent = fixture.intent;
        match field {
            0 => intent.document_id = 99.try_into().unwrap(),
            1 => intent.page.source_id = 99.try_into().unwrap(),
            2 => intent.page.range.end = 99.try_into().unwrap(),
            3 => intent.analysis_id = 99.try_into().unwrap(),
            4 => intent.completed_at_unix_ms = -1,
            5 => intent.departing_position.source_id = 99.try_into().unwrap(),
            6 => intent.departing_position.chapter_id = 99.try_into().unwrap(),
            7 => intent.departing_position.offset = intent.page.range.end,
            _ => intent.departing_position.offset = 99.try_into().unwrap(),
        }
        assert!(!session.begin_completion(identity, intent), "field {field}");
        assert_eq!(session.page().unwrap().identity, identity);
        assert!(!session.completion_state().pending);
    }
    let mut missing = snapshot();
    missing.segmentation = None;
    missing.tokens.clear();
    let request = session.begin_load(base(&missing)).unwrap();
    assert!(session.accept(request, missing));
    assert!(!session.begin_completion(session.page().unwrap().identity, fixture.intent));
}

#[test]
fn preparation_failure_retries_exact_key_and_timestamp_with_fresh_callback_attempts() {
    let fixture = Fixture::new();
    let mut session = EffectiveReaderSession::default();
    let first = fixture.begin(&mut session);
    assert_eq!(
        session.finish_completion_request(&first, Err(DocumentError::Unavailable)),
        Some(Progress::Failed(DocumentError::Unavailable))
    );
    assert!(session.completion_state().pending);
    assert!(!session.completion_state().busy);
    assert_eq!(
        session.completion_state().error,
        Some(DocumentError::Unavailable)
    );
    session.invalidate();
    let retry = session.dispatch_completion().unwrap();
    assert_ne!(retry, first);
    assert_eq!(retry.operation(), first.operation());
    let prepared = fixture.prepared(fixture.intent, 10);
    assert_eq!(
        session.finish_completion_request(
            &first,
            Ok(Output::CompletionPrepared(Arc::clone(&prepared)))
        ),
        None
    );
    assert!(session.completion_state().busy);
    assert_eq!(
        session.finish_completion_request(&retry, Ok(Output::CompletionPrepared(prepared))),
        Some(Progress::Prepared)
    );
    assert!(session.completion_state().error.is_none());
    assert!(!session.completion_state().busy);
    assert!(session
        .finish_completion_request(&retry, Err(DocumentError::Unavailable))
        .is_none());
}

#[test]
fn wrong_stage_or_changed_reservation_cannot_replace_the_accepted_command() {
    let fixture = Fixture::new();
    let mut session = EffectiveReaderSession::default();
    let first = fixture.begin(&mut session);
    assert_eq!(
        session.finish_completion_request(
            &first,
            Ok(Output::PageCompleted(
                fixture.completed_document(),
                AnalyzedCompletionOutcome::AlreadyRecorded
            ))
        ),
        Some(Progress::Failed(DocumentError::InvalidDocument))
    );
    for field in 0..5 {
        let mut intent = fixture.intent;
        match field {
            0 => intent.document_id = 99.try_into().unwrap(),
            1 => intent.page_length = PageLength::Characters500,
            2 => intent.completed_at_unix_ms += 1,
            3 => intent.departing_position.offset = 3.try_into().unwrap(),
            _ => intent.page.range.start = 3.try_into().unwrap(),
        }
        let retry = session.dispatch_completion().unwrap();
        assert_eq!(
            session.finish_completion_request(
                &retry,
                Ok(Output::CompletionPrepared(fixture.prepared(intent, 10)))
            ),
            Some(Progress::Failed(DocumentError::InvalidDocument)),
            "field {field}"
        );
    }
    let mut changed = Fixture::new();
    changed.original.tokens[0].reading = None;
    let retry = session.dispatch_completion().unwrap();
    assert_eq!(
        session.finish_completion_request(
            &retry,
            Ok(Output::CompletionPrepared(
                changed.prepared(fixture.intent, 10)
            ))
        ),
        Some(Progress::Failed(DocumentError::InvalidDocument))
    );
    let retry = session.dispatch_completion().unwrap();
    assert_eq!(
        session.finish_completion_request(
            &retry,
            Ok(Output::CompletionPrepared(
                fixture.prepared(fixture.intent, 10)
            ))
        ),
        Some(Progress::Prepared)
    );
}

#[test]
fn commit_retry_keeps_the_same_reservation_and_emits_success_once() {
    let fixture = Fixture::new();
    for outcome in [
        AnalyzedCompletionOutcome::Recorded { newly_known: 3 },
        AnalyzedCompletionOutcome::Recorded { newly_known: 0 },
        AnalyzedCompletionOutcome::AlreadyRecorded,
    ] {
        let mut session = EffectiveReaderSession::default();
        let first = fixture.prepare(&mut session);
        assert_eq!(
            session.finish_completion_request(&first, Err(DocumentError::Unavailable)),
            Some(Progress::Failed(DocumentError::Unavailable))
        );
        session.invalidate();
        let retry = session.dispatch_completion().unwrap();
        assert_ne!(retry, first);
        let (Op::CompletePage(a), Op::CompletePage(b)) = (first.operation(), retry.operation())
        else {
            panic!("commit only")
        };
        assert!(Arc::ptr_eq(&a, &b));

        assert_eq!(
            b.occurrences()
                .iter()
                .map(|token| token.surface.as_str())
                .collect::<Vec<_>>(),
            ["猫", "犬", "𠮷か\u{3099}\0"]
        );
        assert!(b.occurrences()[0].metadata.is_some());
        let output = Output::PageCompleted(fixture.completed_document(), outcome);
        assert!(session
            .finish_completion_request(&first, Ok(output.clone()))
            .is_none());
        let Some(Progress::Completed {
            completion,
            document,
            outcome: returned,
        }) = session.finish_completion_request(&retry, Ok(output.clone()))
        else {
            panic!("completed")
        };
        assert_eq!(completion, b.completion());
        assert_eq!(*document, fixture.completed_document());
        assert_eq!(returned, outcome);
        assert!(session
            .finish_completion_request(&retry, Ok(output))
            .is_none());
        assert!(!session.completion_state().pending);
        assert!(!session.completion_state().busy);
        assert!(session.dispatch_completion().is_none());
        // A deliberate reread needs a fresh accepted page and reservation.
        let identity = fixture.load(&mut session);
        assert!(session.begin_completion(identity, fixture.intent));
        assert!(matches!(
            session.dispatch_completion().unwrap().operation(),
            Op::PrepareCompletion { .. }
        ));
        // A new process/session never auto-replays an unfinished reservation.
        assert!(EffectiveReaderSession::default()
            .dispatch_completion()
            .is_none());
    }
}

#[test]
fn invalid_commit_document_keeps_exact_retry_obligation() {
    let fixture = Fixture::new();
    let mut session = EffectiveReaderSession::default();
    let mut request = fixture.prepare(&mut session);
    for field in 0..3 {
        let mut document = fixture.completed_document();
        match field {
            0 => document.document_id = 99.try_into().unwrap(),
            1 => document.chapters.clear(),
            _ => document.chapters[0].text.clear(),
        }
        assert_eq!(
            session.finish_completion_request(
                &request,
                Ok(Output::PageCompleted(
                    document,
                    AnalyzedCompletionOutcome::AlreadyRecorded
                ))
            ),
            Some(Progress::Failed(DocumentError::InvalidDocument))
        );
        let retry = session.dispatch_completion().unwrap();
        assert_eq!(retry.operation(), request.operation());
        request = retry;
    }
    assert!(matches!(
        session.finish_completion_request(
            &request,
            Ok(Output::PageCompleted(
                fixture.completed_document(),
                AnalyzedCompletionOutcome::AlreadyRecorded
            ))
        ),
        Some(Progress::Completed { .. })
    ));
}
