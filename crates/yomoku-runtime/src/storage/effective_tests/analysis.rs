use super::*;
use crate::imports::PasteAnalyzer;
use yomoku_core::{
    effective_overrides::EffectiveOverrideFailure as Failure,
    effective_session::{EffectiveOverrideProgress as Progress, EffectiveOverrideRequest},
};

struct AlternateAnalyzer {
    worker: thread::ThreadId,
    trace: mpsc::Sender<(&'static str, thread::ThreadId)>,
    invalid: bool,
}

impl PasteAnalyzer for AlternateAnalyzer {
    fn analyze(&mut self, _: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        panic!("generation must request an explicit mode")
    }

    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        assert_eq!(thread::current().id(), self.worker);
        self.trace.send(("analyze", self.worker)).unwrap();
        let mut output = analysis(chapter).unwrap();
        assert_eq!(&output.profile, profile);
        assert_eq!(chapter.text, "食べた 食べた 猫 English 。 𠮷か\u{3099}\0");
        output.mode = mode;
        output.tokens = output
            .tokens
            .into_iter()
            .flat_map(|token| {
                if token.surface != "食べた" {
                    return vec![token];
                }
                let start = token.anchor.range.start.byte_index(&chapter.text).unwrap();
                [
                    (start, start + "食べ".len()),
                    (start + "食べ".len(), start + "食べた".len()),
                ]
                .map(|(start, end)| {
                    let surface = chapter.text[start..end].to_owned();
                    AnalyzedToken {
                        anchor: TextAnchor {
                            range: TextRange {
                                start: TextOffset::at(&chapter.text, start).unwrap(),
                                end: TextOffset::at(&chapter.text, end).unwrap(),
                            },
                            ..token.anchor
                        },
                        reading: Some(format!("alternate{surface}")),
                        dictionary_form: Some(Surface::new(surface.clone()).unwrap()),
                        surface,
                        is_japanese: true,
                    }
                })
                .into()
            })
            .collect();
        if self.invalid {
            output.profile.engine_revision.push_str("-wrong");
        }
        Ok(output)
    }
}

impl Drop for AlternateAnalyzer {
    fn drop(&mut self) {
        assert_eq!(thread::current().id(), self.worker);
        let _ = self.trace.send(("drop", self.worker));
    }
}

fn prompt(router: &Router, session: &mut EffectiveReaderSession, base: ReaderTokenKey) {
    load(router, session, base);
    let identity = session.page().unwrap().identity;
    let anchor = TextAnchor {
        range: TextRange {
            start: TextOffset::START,
            end: 9.try_into().unwrap(),
        },
        ..base.page
    };
    let request = session.begin_override_load(identity, anchor).unwrap();
    let StorageResult::Effective(output) = run(router, request.operation()) else {
        panic!("context")
    };
    assert_eq!(
        session.finish_override_request(&request, Ok(output)),
        Some(Progress::Loaded)
    );
}

fn finish(
    router: &Router,
    session: &mut EffectiveReaderSession,
    request: &EffectiveOverrideRequest,
) -> Progress {
    let StorageResult::Effective(output) = run(router, request.operation()) else {
        panic!("effective result")
    };
    session
        .finish_override_request(request, Ok(output))
        .unwrap()
}

#[test]
fn effective_worker_on_demand_analysis_is_lazy_retries_loader_and_sql_failures_and_requires_reload_after_lost_success(
) {
    let fixture = Fixture::new();
    let mut storage = fixture.storage();
    let pending = storage
        .load_pending_reading_time(fixture.base.document_id, fixture.intent.departing_position)
        .unwrap();
    storage
        .save_pending_reading_time(pending, PendingReadingTime::from_millis(987).unwrap())
        .unwrap();
    let document = storage
        .load_document_reader_snapshot(fixture.base.document_id)
        .unwrap();
    drop(storage);
    let state = || {
        fixture.sql().query_row(
        "SELECT (SELECT COUNT(*) FROM vocabulary), (SELECT COUNT(*) FROM vocabulary_status_events),
         (SELECT COUNT(*) FROM reading_occurrences), (SELECT SUM(milliseconds) FROM pending_reading_time),
         (SELECT COUNT(*) FROM segmentation_overrides)", [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))
    ).unwrap()
    };
    let before_state = state();
    let (sender, trace) = mpsc::channel();
    let loads = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::clone(&loads);
    let (effects, lifecycle) = StorageEffects::with_dictionaries(
        fixture.path(),
        Box::new(move || {
            sender.send(("load", thread::current().id())).unwrap();
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(DocumentError::Unavailable);
            }
            Ok(Box::new(AlternateAnalyzer {
                worker: thread::current().id(),
                trace: sender.clone(),
                invalid: false,
            }))
        }),
        Box::new(|| panic!("generation must not open word dictionaries")),
    )
    .unwrap();
    let router = Core::<Probe>::new().handle_effects_using(effects);
    let mut session = EffectiveReaderSession::default();
    prompt(&router, &mut session, fixture.base);
    assert!(trace.try_recv().is_err());
    let request = session.begin_override_analysis(SplitMode::A).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert_eq!(
        finish(&router, &mut session, &request),
        Progress::Failed(Failure::Storage(DocumentError::Unavailable))
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    fixture.sql().execute_batch("CREATE TRIGGER fail_analysis AFTER INSERT ON analyzed_tokens BEGIN SELECT RAISE(ABORT, 'analysis rollback'); END;").unwrap();
    let retry = session.retry_override_write().unwrap();
    assert_eq!(retry.operation(), request.operation());
    assert_eq!(
        finish(&router, &mut session, &retry),
        Progress::Failed(Failure::Storage(DocumentError::Unavailable))
    );
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_analysis")
        .unwrap();
    let retry = session.retry_override_write().unwrap();
    assert!(matches!(
        run(&router, retry.operation()),
        StorageResult::Effective(Output::OverrideAnalysisStored(SplitMode::A))
    ));
    // Lose success, then reopen. Its newly resolved metadata retires the key.
    session
        .finish_override_request(&retry, Err(DocumentError::Unavailable))
        .unwrap();
    drop(router);
    let router = fixture.override_router();
    let retry = session.retry_override_write().unwrap();
    assert_eq!(
        finish(&router, &mut session, &retry),
        Progress::Failed(Failure::StaleContext)
    );
    assert!(session.override_state().reload_required);
    assert!(session.retry_override_write().is_none());
    assert_eq!(state(), before_state);
    assert_eq!(
        fixture
            .storage()
            .load_document_reader_snapshot(fixture.base.document_id)
            .unwrap(),
        document
    );
    assert_eq!(loads.load(Ordering::SeqCst), 2);
    // All asset loading, generation, and dropping stayed on the same FIFO worker.
    let observed = trace.iter().collect::<Vec<_>>();
    assert_eq!(
        observed.iter().map(|(event, _)| *event).collect::<Vec<_>>(),
        ["load", "load", "analyze", "analyze", "drop"]
    );
    assert!(observed
        .iter()
        .all(|(_, worker)| *worker == observed[0].1 && *worker != thread::current().id()));
    assert!(lifecycle.set_foreground(true).is_err());
    prompt(&router, &mut session, fixture.base);
    assert_eq!(
        session.override_context().unwrap().choices()[0].mode,
        SplitMode::A
    );
    // A fresh stored request is served even by a worker with no analyzer capability.
    let context = Arc::new(session.override_context().unwrap().clone());
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert!(matches!(
        run(
            &router,
            Op::GenerateOverrideAnalysis {
                context,
                mode: SplitMode::A
            }
        ),
        StorageResult::Effective(Output::OverrideAnalysisStored(SplitMode::A))
    ));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
}

#[test]
fn effective_worker_on_demand_analysis_rejects_foreign_profile_and_drops_bad_analyzer_for_explicit_retry(
) {
    let fixture = Fixture::new();
    let (sender, trace) = mpsc::channel();
    let (effects, _) = StorageEffects::with_dictionaries(
        fixture.path(),
        Box::new(move || {
            sender.send(("load", thread::current().id())).unwrap();
            Ok(Box::new(AlternateAnalyzer {
                worker: thread::current().id(),
                trace: sender.clone(),
                invalid: true,
            }))
        }),
        Box::new(|| panic!("no dictionary")),
    )
    .unwrap();
    let router = Core::<Probe>::new().handle_effects_using(effects);
    let mut session = EffectiveReaderSession::default();
    prompt(&router, &mut session, fixture.base);
    let request = session.begin_override_analysis(SplitMode::C).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    for _ in 0..2 {
        let request = if session.override_state().pending && !session.override_state().busy {
            session.retry_override_write().unwrap()
        } else {
            request.clone()
        };
        assert_eq!(
            finish(&router, &mut session, &request),
            Progress::Failed(Failure::Storage(DocumentError::InvalidDocument))
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap().0, "load");
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap().0, "analyze");
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap().0, "drop");
    }
    assert!(session.cancel_override_retry());
    assert!(session.override_state().reload_required);
}

#[test]
fn effective_worker_on_demand_modes_share_import_analyzer_cache_and_background_release() {
    let fixture = Fixture::new();
    let (sender, trace) = mpsc::channel();
    let (effects, lifecycle) = StorageEffects::with_dictionaries(
        fixture.path(),
        Box::new(move || {
            sender.send(("load", thread::current().id())).unwrap();
            Ok(Box::new(AlternateAnalyzer {
                worker: thread::current().id(),
                trace: sender.clone(),
                invalid: false,
            }))
        }),
        Box::new(|| panic!("no dictionary")),
    )
    .unwrap();
    let router = Core::<Probe>::new().handle_effects_using(effects);
    let mut session = EffectiveReaderSession::default();
    for mode in [SplitMode::A, SplitMode::C] {
        prompt(&router, &mut session, fixture.base);
        let request = session.begin_override_analysis(mode).unwrap();
        assert_eq!(
            finish(&router, &mut session, &request),
            Progress::AnalysisStored(mode)
        );
        assert!(session.override_state().reload_required);
    }
    let loaded = trace.recv_timeout(TIMEOUT).unwrap();
    assert_eq!(loaded.0, "load");
    for _ in 0..2 {
        assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), ("analyze", loaded.1));
    }
    assert!(trace.try_recv().is_err());
    lifecycle.set_foreground(false).unwrap();
    assert_eq!(trace.recv_timeout(TIMEOUT).unwrap(), ("drop", loaded.1));
    // Cached alternates keep working hidden, without extraction/loading again.
    prompt(&router, &mut session, fixture.base);
    assert_eq!(session.override_context().unwrap().choices().len(), 3);
    assert!(trace.try_recv().is_err());
}

#[test]
fn effective_worker_import_only_analyzer_cannot_silently_supply_mode_b_for_alternates() {
    let fixture = Fixture::new();
    let (effects, _) = StorageEffects::with_analyzed_imports(
        fixture.path(),
        Box::new(|| {
            Ok(Box::new(
                |_: &ChapterText| -> Result<ChapterAnalysis, DocumentError> {
                    panic!("alternate must not call default B")
                },
            ))
        }),
    )
    .unwrap();
    let router = Core::<Probe>::new().handle_effects_using(effects);
    let mut session = EffectiveReaderSession::default();
    prompt(&router, &mut session, fixture.base);
    let request = session.begin_override_analysis(SplitMode::A).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert_eq!(
        finish(&router, &mut session, &request),
        Progress::Failed(Failure::Storage(DocumentError::Unavailable))
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
}
