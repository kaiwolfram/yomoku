//! Exercise the real application rather than the isolated retained component.
use super::*;
use yomoku_core::app::{ContentRoute, Event as AppEvent, Model, ViewModel, Yomoku};

type AppRouter = HandleEffectLayer<Core<Yomoku>, StorageEffects>;

mod library;

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
fn effective_application_fifo_custom_lookup_save_retry_delete_and_builtin_gates() {
    let fixture = Fixture::new();
    let router = word_router(&fixture);
    let view = load_app(&router, &fixture);
    let selection = app_selection(&view, "食べた");
    let before = std::fs::read(fixture.path()).unwrap();
    let view = dispatch(&router, AppEvent::LoadEffectiveReaderWord(selection));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    let word = view.effective_reader_word.unwrap().details.unwrap();
    assert!(
        word.token.vocabulary.is_some()
            && word.token.reading.is_some()
            && word.token.dictionary_form.is_some()
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
            "SELECT english_meaning, reading FROM custom_dictionary_entries WHERE surface = '食べた'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .unwrap();
    assert_eq!(stored, (meaning.into(), reading.into()));
    assert!(fixture
        .storage()
        .vocabulary_entry(&Surface::new("食べた").unwrap())
        .unwrap()
        .is_some());

    // A fresh app uses the committed literal entry without loading an analyzer.
    let fresh = word_router(&fixture);
    let view = load_app(&fresh, &fixture);
    let selected = app_selection(&view, "食べた");
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
