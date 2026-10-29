//! Exercise the real application rather than the isolated retained component.
use super::*;
use yomoku_core::{
    active_time::{ReaderClockAction, ReaderClockSample},
    app::{ContentRoute, Event as AppEvent, Model, ViewModel, Yomoku},
};

type AppRouter = HandleEffectLayer<Core<Yomoku>, StorageEffects>;

fn router(fixture: &Fixture) -> AppRouter {
    let (effects, _) = StorageEffects::with_dictionaries(
        fixture.path(),
        Box::new(|| panic!("application reads/completion must not load Sudachi")),
        Box::new(|| panic!("application reads/completion must not load word dictionaries")),
    )
    .unwrap();
    Core::new_with(Yomoku, Model::with_effective_reader()).handle_effects_using(effects)
}

fn dispatch(router: &AppRouter, event: AppEvent) -> ViewModel {
    let (sender, receiver) = mpsc::channel();
    let effects = router.update(event, move |effects| {
        assert!(effects
            .iter()
            .all(|effect| matches!(effect, Effect::Render(_))));
        sender.send(()).unwrap();
    });
    assert!(effects
        .iter()
        .all(|effect| matches!(effect, Effect::Render(_))));
    let deadline = std::time::Instant::now() + TIMEOUT;
    loop {
        let view = router.view();
        if !view.storage_busy {
            return view;
        }
        receiver
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
    }
}

fn load_app(router: &AppRouter, fixture: &Fixture) -> ViewModel {
    let view = dispatch(router, AppEvent::OpenReader(fixture.base.document_id));
    assert_eq!(view.content_route, ContentRoute::Reader);
    let page = view.reader.unwrap();
    dispatch(
        router,
        AppEvent::LoadReaderTokens(ReaderTokenKey {
            document_id: page.document_id,
            page: page.anchor,
            analysis: view.reader_analysis.unwrap(),
        }),
    )
}

#[test]
fn effective_application_fifo_preserves_frozen_completion_after_failed_commit_and_rule_reversal() {
    let fixture = Fixture::new();
    let router = router(&fixture);
    let view = load_app(&router, &fixture);
    let tokens = view.effective_reader_tokens.unwrap();
    assert_eq!(
        tokens
            .tokens
            .iter()
            .map(|token| token.surface.as_str())
            .collect::<Vec<_>>(),
        ["食べ", "た", "食べ", "た", "猫", "𠮷か\u{3099}\0"]
    );
    assert!(tokens.tokens[0].reading.is_none() && tokens.tokens[1].reading.is_none());
    let page = view.reader.unwrap();
    dispatch(&router, AppEvent::RestoreReadingTime);
    for (monotonic_ms, action) in [
        (0, ReaderClockAction::Resume),
        (1250, ReaderClockAction::Pause),
    ] {
        dispatch(
            &router,
            AppEvent::ObserveReaderClock(ReaderClockSample {
                document_id: page.document_id,
                position: page.position,
                monotonic_ms,
                action,
            }),
        );
    }
    fixture.sql().execute_batch("CREATE TRIGGER fail_app_completion BEFORE INSERT ON reading_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let view = dispatch(
        &router,
        AppEvent::CompleteEffectiveReaderPage {
            identity: tokens.identity,
            position: page.position,
            completed_at_unix_ms: 500,
        },
    );
    assert!(view.completion_save_pending && !view.storage_busy);
    assert!(view.document_error.is_some() && view.effective_reader_tokens.is_none());
    let sql = fixture.sql();
    assert_eq!(
        sql.query_row(
            "SELECT COUNT(*) FROM effective_completion_commands",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    sql.execute_batch("DROP TRIGGER fail_app_completion;")
        .unwrap();
    let view = dispatch(&router, AppEvent::RetryReaderCompletion);
    assert_eq!(view.content_route, ContentRoute::Main);
    assert!(!view.completion_save_pending && !view.storage_busy);
    assert_eq!(view.completion_notice.unwrap().newly_known, 4);
    assert!(view.book_completion_notice.is_some());
    assert_eq!(
        sql.query_row(
            "SELECT COUNT(*) FROM effective_completion_commands",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        sql.query_row("SELECT active_time_ms FROM reading_completions", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        1250
    );
    assert!(
        fixture
            .storage()
            .load_document_reader_snapshot(fixture.base.document_id)
            .unwrap()
            .unwrap()
            .document
            .finished_at_least_once
    );

    // Fresh native app construction never replays a reservation. Explicit reopen
    // resets a finished book, and a deliberate reread resolves the reversed rule.
    let fresh = self::router(&fixture);
    assert!(!fresh.view().completion_save_pending);
    let view = load_app(&fresh, &fixture);
    let page = view.reader.unwrap();
    assert_eq!(page.position.offset, TextOffset::START);
    let tokens = view.effective_reader_tokens.unwrap();
    assert_eq!(tokens.tokens[0].surface, "食べた");
    let view = dispatch(
        &fresh,
        AppEvent::CompleteEffectiveReaderPage {
            identity: tokens.identity,
            position: page.position,
            completed_at_unix_ms: 600,
        },
    );
    assert_eq!(view.completion_notice.unwrap().newly_known, 1);
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        sql.query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
}
