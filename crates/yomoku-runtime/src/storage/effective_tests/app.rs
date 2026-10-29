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

fn word_router(fixture: &Fixture) -> AppRouter {
    let index = fixture.index();
    let (effects, _) = StorageEffects::with_dictionaries(
        fixture.path(),
        Box::new(|| panic!("effective application words must not load Sudachi")),
        Box::new(move || Ok(index.clone())),
    )
    .unwrap();
    Core::new_with(Yomoku, Model::with_effective_reader()).handle_effects_using(effects)
}

fn app_selection(view: &ViewModel, surface: &str) -> EffectiveReaderSelection {
    let page = view.effective_reader_tokens.as_ref().unwrap();
    EffectiveReaderSelection {
        page: page.identity,
        segment_index: page
            .tokens
            .iter()
            .find(|token| token.surface == surface)
            .unwrap()
            .segment_index,
    }
}

#[test]
fn effective_application_fifo_override_rollback_exact_retry_reload_and_reverse() {
    use yomoku_core::effective_session::EffectiveOverrideAction as Action;
    let fixture = Fixture::new();
    let original_position = fixture
        .storage()
        .load_document_reader_snapshot(fixture.base.document_id)
        .unwrap()
        .unwrap()
        .document
        .reading_position;
    let router = router(&fixture); // Stored choices/manual edits load neither dictionary.
    let view = load_app(&router, &fixture);
    let open = |view: &ViewModel| {
        AppEvent::EffectiveReaderOverride(Action::Open {
            page: view.effective_reader_tokens.as_ref().unwrap().identity,
            anchor: TextAnchor {
                range: TextRange {
                    start: TextOffset::START,
                    end: ("食べた".len() as i64).try_into().unwrap(),
                },
                ..fixture.base.page
            },
        })
    };
    let reload = || dispatch(&router, AppEvent::LoadReaderTokens(fixture.base));
    let before = std::fs::read(fixture.path()).unwrap();
    let view = dispatch(&router, open(&view));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let prompt = view.effective_reader_override.unwrap().prompt.unwrap();
    assert_eq!(prompt.default_scope, OverrideScope::AllOccurrences);
    assert_eq!(prompt.choices.len(), 1);
    assert_eq!(prompt.choices[0].mode, SplitMode::B);
    assert_eq!(prompt.winning_rules[0].id, fixture.rule);
    fixture.sql().execute_batch("CREATE TRIGGER fail_app_override BEFORE INSERT ON segmentation_overrides BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let view = dispatch(
        &router,
        AppEvent::EffectiveReaderOverride(Action::Choose {
            identity: prompt.identity,
            mode: SplitMode::B,
            scope: OverrideScope::ThisOccurrence,
            registered_at_unix_ms: 601,
        }),
    );
    assert!(view.effective_reader_override.unwrap().pending);
    assert!(view.effective_reader_tokens.is_none());
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM segmentation_overrides", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_app_override")
        .unwrap();
    let view = dispatch(
        &router,
        AppEvent::EffectiveReaderOverride(Action::Retry {
            identity: prompt.identity,
        }),
    );
    let state = view.effective_reader_override.unwrap();
    assert!(state.reload_required && !state.pending);
    assert!(view.effective_reader_tokens.is_none());
    let view = reload();
    assert_eq!(
        view.effective_reader_tokens.as_ref().unwrap().tokens[0].surface,
        "食べた"
    );
    assert_eq!(
        view.effective_reader_tokens.as_ref().unwrap().tokens[1].surface,
        "食べ"
    );
    let view = dispatch(&router, open(&view));
    let prompt = view.effective_reader_override.unwrap().prompt.unwrap();
    let local = prompt
        .winning_rules
        .iter()
        .find(|r| r.scope == OverrideScope::ThisOccurrence)
        .unwrap()
        .id;
    dispatch(
        &router,
        AppEvent::EffectiveReaderOverride(Action::Reverse {
            identity: prompt.identity,
            id: local,
        }),
    );
    let view = reload();
    assert_eq!(
        view.effective_reader_tokens.as_ref().unwrap().tokens[0].surface,
        "食べ"
    );

    // Literal manual parts persist through a new app/storage connection.
    let page = view.effective_reader_tokens.as_ref().unwrap();
    let token = page
        .tokens
        .iter()
        .find(|t| t.surface == "𠮷か\u{3099}\0")
        .unwrap();
    let view = dispatch(
        &router,
        AppEvent::EffectiveReaderOverride(Action::Open {
            page: page.identity,
            anchor: token.anchor,
        }),
    );
    let identity = view.effective_reader_override.unwrap().identity.unwrap();
    let view = dispatch(
        &router,
        AppEvent::EffectiveReaderOverride(Action::Save {
            identity,
            scope: OverrideScope::AllOccurrences,
            parts: vec!["𠮷".into(), "か\u{3099}\0".into()],
            registered_at_unix_ms: 602,
        }),
    );
    assert!(!view.effective_reader_override.unwrap().pending);
    let fresh = self::router(&fixture);
    let view = load_app(&fresh, &fixture);
    let tokens = view.effective_reader_tokens.unwrap().tokens;
    assert!(tokens.iter().any(|t| t.surface == "𠮷"));
    assert!(tokens.iter().any(|t| t.surface == "か\u{3099}\0"));
    let sql = fixture.sql();
    for table in ["reading_completions", "reading_occurrences"] {
        assert_eq!(
            sql.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        sql.query_row(
            "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        fixture
            .storage()
            .load_document_reader_snapshot(fixture.base.document_id)
            .unwrap()
            .unwrap()
            .document
            .reading_position,
        original_position
    );
}

#[test]
fn effective_application_fifo_custom_lookup_save_retry_delete_and_builtin_gates() {
    let fixture = Fixture::new();
    let router = word_router(&fixture);
    let view = load_app(&router, &fixture);
    let selection = app_selection(&view, "食べ");
    let before = std::fs::read(fixture.path()).unwrap();
    let view = dispatch(&router, AppEvent::LoadEffectiveReaderWord(selection));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let word = view.effective_reader_word.unwrap().details.unwrap();
    assert!(
        word.token.vocabulary.is_none()
            && word.token.reading.is_none()
            && word.token.dictionary_form.is_none()
    );
    assert_eq!(word.encountered, 0);
    assert!(word.candidates.words.is_empty() && word.candidates.names.is_empty());
    let meaning = " literal\0meaning 📖 ";
    let reading = " よみか\u{3099}\0 ";
    fixture.sql().execute_batch("CREATE TRIGGER fail_app_custom BEFORE INSERT ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let view = dispatch(
        &router,
        AppEvent::SaveEffectiveReaderCustomEntry {
            selection,
            english_meaning: meaning.into(),
            reading: reading.into(),
        },
    );
    let word = view.effective_reader_word.unwrap();
    assert!(word.custom_entry_pending && word.custom_entry_error.is_some());
    assert!(word.details.unwrap().custom_entry.is_none());
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM custom_dictionary_entries", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_app_custom")
        .unwrap();
    let view = dispatch(&router, AppEvent::RetryEffectiveReaderCustomEntry);
    let word = view.effective_reader_word.unwrap();
    assert!(!word.custom_entry_pending && word.custom_entry_error.is_none());
    let entry = word.details.unwrap().custom_entry.unwrap();
    assert_eq!(entry.english_meaning, meaning);
    assert_eq!(entry.reading, reading);
    let stored = fixture
        .sql()
        .query_row(
            "SELECT english_meaning, reading FROM custom_dictionary_entries WHERE surface = '食べ'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .unwrap();
    assert_eq!(stored, (meaning.into(), reading.into()));
    assert!(fixture
        .storage()
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .is_none());

    // A fresh app uses the committed literal entry without loading an analyzer.
    let fresh = word_router(&fixture);
    let view = load_app(&fresh, &fixture);
    let selected = app_selection(&view, "食べ");
    let view = dispatch(&fresh, AppEvent::LoadEffectiveReaderWord(selected));
    assert_eq!(
        view.effective_reader_word
            .unwrap()
            .details
            .unwrap()
            .custom_entry
            .unwrap()
            .english_meaning,
        meaning
    );
    fixture.sql().execute_batch("CREATE TRIGGER fail_app_delete BEFORE DELETE ON custom_dictionary_entries BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let view = dispatch(&fresh, AppEvent::DeleteEffectiveReaderCustomEntry(selected));
    let word = view.effective_reader_word.unwrap();
    assert!(word.custom_entry_pending && word.custom_entry_error.is_some());
    assert!(word.details.unwrap().custom_entry.is_some());
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_app_delete")
        .unwrap();
    let view = dispatch(&fresh, AppEvent::RetryEffectiveReaderCustomEntry);
    assert!(view
        .effective_reader_word
        .unwrap()
        .details
        .unwrap()
        .custom_entry
        .is_none());

    let selected = app_selection(&fresh.view(), "猫");
    let view = dispatch(&fresh, AppEvent::LoadEffectiveReaderWord(selected));
    assert!(!view
        .effective_reader_word
        .unwrap()
        .details
        .unwrap()
        .candidates
        .words
        .is_empty());
    let before = std::fs::read(fixture.path()).unwrap();
    dispatch(
        &fresh,
        AppEvent::SaveEffectiveReaderCustomEntry {
            selection: selected,
            english_meaning: meaning.into(),
            reading: reading.into(),
        },
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_completions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        fixture
            .sql()
            .query_row(
                "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn effective_application_fifo_status_rollback_exact_retry_stale_rules_and_cancel() {
    let fixture = Fixture::new();
    let router = router(&fixture); // Both dictionary loaders panic if invoked.
    let view = load_app(&router, &fixture);
    let selected = app_selection(&view, "猫");
    let id = view
        .effective_reader_tokens
        .unwrap()
        .tokens
        .iter()
        .find(|token| token.surface == "猫")
        .unwrap()
        .vocabulary
        .unwrap()
        .id;
    let choose = |status| AppEvent::SetEffectiveReaderStatus {
        selection: selected,
        vocabulary_id: id,
        status,
        occurred_at_unix_ms: 600,
    };
    fixture.sql().execute_batch("CREATE TRIGGER fail_app_status BEFORE INSERT ON vocabulary_status_events WHEN NEW.cause = 'manual' BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
    let view = dispatch(&router, choose(VocabularyStatus::Learning));
    assert!(view.effective_reader_status.unwrap().pending);
    assert!(view.effective_reader_status.unwrap().error.is_some());
    assert_eq!(
        fixture
            .storage()
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_app_status")
        .unwrap();
    let view = dispatch(&router, AppEvent::RetryEffectiveReaderStatus);
    assert!(!view.effective_reader_status.unwrap().pending);
    assert_eq!(
        fixture
            .storage()
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Learning
    );
    for status in [
        VocabularyStatus::Learning,
        VocabularyStatus::Familiar,
        VocabularyStatus::Known,
        VocabularyStatus::Ignored,
        VocabularyStatus::New,
    ] {
        let view = dispatch(&router, choose(status));
        assert!(!view.effective_reader_status.unwrap().pending);
    }
    assert_eq!(
        fixture
            .sql()
            .query_row(
                "SELECT COUNT(*) FROM vocabulary_status_events WHERE cause = 'manual'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        5
    );
    assert_eq!(
        fixture
            .sql()
            .query_row("SELECT COUNT(*) FROM reading_occurrences", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );

    // A changed partition cannot silently rebind the frozen selection on retry.
    fixture
        .storage()
        .reverse_segmentation_override(fixture.rule)
        .unwrap();
    let view = dispatch(&router, choose(VocabularyStatus::Known));
    assert!(view.effective_reader_status.unwrap().pending);
    dispatch(&router, AppEvent::CloseDocument);
    assert_eq!(router.view().content_route, ContentRoute::Reader);
    let view = dispatch(&router, AppEvent::RetryEffectiveReaderStatus);
    assert!(view.effective_reader_status.unwrap().pending);
    assert_eq!(
        fixture
            .storage()
            .vocabulary_entry(&Surface::new("猫").unwrap())
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::New
    );
    let view = dispatch(&router, AppEvent::CancelEffectiveReaderStatus);
    assert_eq!(view.content_route, ContentRoute::Main);
    assert!(!view.effective_reader_status.unwrap().pending);
    let view = load_app(&router, &fixture);
    assert_eq!(
        view.effective_reader_tokens.unwrap().tokens[0].surface,
        "食べた"
    );
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
