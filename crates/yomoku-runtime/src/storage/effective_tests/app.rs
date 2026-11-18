//! Exercise the real application rather than the isolated retained component.
use super::*;
use yomoku_core::app::{Event as AppEvent, Model, ViewModel, Yomoku};

type AppRouter = HandleEffectLayer<Core<Yomoku>, StorageEffects>;

mod library;

fn router(fixture: &Fixture) -> AppRouter {
    let (effects, _) = StorageEffects::new(
        fixture.path(),
        Box::new(|| panic!("application reads/completion must not load Sudachi")),
        Box::new(|| panic!("application reads/completion must not load word dictionaries")),
    )
    .unwrap();
    Core::new_with(Yomoku, Model::default()).handle_effects_using(effects)
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

#[test]
fn post_commit_reload_failure_retries_the_same_receipt_without_a_second_notice() {
    let fixture = Fixture::new();
    let sql = fixture.sql();
    sql.execute_batch(
        "CREATE TRIGGER invalidate_completion_snapshot AFTER INSERT ON reading_completions
         BEGIN UPDATE chapter_texts SET text = '破損' WHERE chapter_id = NEW.chapter_id; END;",
    )
    .unwrap();
    let router = router(&fixture);
    dispatch(&router, AppEvent::RefreshLibrary);
    let opened = dispatch(&router, AppEvent::OpenReader(fixture.base.document_id));
    let position = opened.reader.unwrap().position;
    let identity = dispatch(&router, AppEvent::LoadReaderTokens(fixture.base))
        .effective_reader_tokens
        .unwrap()
        .identity;
    let failed = dispatch(
        &router,
        AppEvent::CompleteEffectiveReaderPage {
            identity,
            position,
            completed_at_unix_ms: 600,
        },
    );
    assert!(failed.document_error.is_some());
    assert!(failed.completion_save_pending);
    assert!(failed.completion_notice.is_none() && failed.book_completion_notice.is_none());
    let counts = || {
        sql.query_row(
            "SELECT (SELECT COUNT(*) FROM reading_completions),
                (SELECT COUNT(*) FROM reading_occurrences),
                (SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'completion'),
                (SELECT byte_offset FROM reading_positions)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .unwrap()
    };
    let text = &fixture.imported.document.chapter.text;
    let recorded = counts();
    assert_eq!(recorded, (1, 4, 3, text.len() as i64));
    sql.execute("UPDATE chapter_texts SET text = ?1", [text])
        .unwrap();
    let retried = dispatch(&router, AppEvent::RetryReaderCompletion);
    assert_eq!(retried.document_error, None);
    assert!(!retried.completion_save_pending);
    assert_eq!(retried.content_route, yomoku_core::app::ContentRoute::Main);
    assert!(retried.completion_notice.is_none() && retried.book_completion_notice.is_none());
    assert_eq!(counts(), recorded);
    assert!(
        retried
            .documents
            .iter()
            .find(|row| row.document_id == fixture.base.document_id)
            .unwrap()
            .is_read
    );
}
