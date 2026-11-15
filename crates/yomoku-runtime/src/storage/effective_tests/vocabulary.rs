//! Vocabulary resolves saved surfaces on the worker without a source document.
use super::*;
use crate::{AnalyzerLoader, PasteAnalyzer};
use yomoku_core::{
    app::{Destination, Event as AppEvent, ViewModel, Yomoku},
    completion::PageCompletionIntent,
    vocabulary::{VocabularyQuery, VocabularyWordStatusCommand},
};

type AppRouter = HandleEffectLayer<Core<Yomoku>, StorageEffects>;

struct WordAnalyzer(Arc<AtomicUsize>);
impl PasteAnalyzer for WordAnalyzer {
    fn analyze(&mut self, chapter: &ChapterText) -> Result<ChapterAnalysis, DocumentError> {
        if self.0.load(Ordering::Relaxed) != 0 {
            return Err(DocumentError::Unavailable);
        }
        let mut result = analysis(chapter).unwrap();
        for token in &mut result.tokens {
            if token.surface == "食べた" {
                token.dictionary_form = Some(Surface::new("食べる").unwrap());
            }
        }
        Ok(result)
    }
    fn analyze_mode(
        &mut self,
        chapter: &ChapterText,
        mode: SplitMode,
        profile: &AnalysisProfile,
    ) -> Result<ChapterAnalysis, DocumentError> {
        let mut result = self.analyze(chapter)?;
        assert_eq!(&result.profile, profile);
        result.mode = mode;
        if chapter.text == "東京都" {
            let parts: &[&str] = match mode {
                SplitMode::A => &["東", "京", "都"],
                _ => &["東京", "都"],
            };
            let mut cursor = 0;
            result.tokens = parts
                .iter()
                .map(|surface| {
                    let start = cursor;
                    cursor += surface.len();
                    AnalyzedToken {
                        anchor: TextAnchor {
                            source_id: chapter.source_id,
                            chapter_id: chapter.chapter_id,
                            range: TextRange {
                                start: TextOffset::at(&chapter.text, start).unwrap(),
                                end: TextOffset::at(&chapter.text, cursor).unwrap(),
                            },
                        },
                        surface: (*surface).to_owned(),
                        is_japanese: true,
                        reading: None,
                        dictionary_form: None,
                    }
                })
                .collect();
        }
        Ok(result)
    }
}

fn router(
    fixture: &Fixture,
    failure: Arc<AtomicUsize>,
    dictionary_failure: Arc<AtomicUsize>,
) -> AppRouter {
    let index = fixture.index();
    let loader: AnalyzerLoader = Box::new(move || Ok(Box::new(WordAnalyzer(failure.clone()))));
    let (effects, _) = StorageEffects::new(
        fixture.path(),
        loader,
        Box::new(move || {
            if dictionary_failure.load(Ordering::Relaxed) != 0 {
                Err(DocumentError::Unavailable)
            } else {
                Ok(index.clone())
            }
        }),
    )
    .unwrap();
    Core::<Yomoku>::new().handle_effects_using(effects)
}

fn dispatch(router: &AppRouter, event: AppEvent) -> ViewModel {
    let (sender, receiver) = mpsc::channel();
    router.update(event, move |_| {
        let _ = sender.send(());
    });
    let deadline = std::time::Instant::now() + TIMEOUT;
    while router.view().storage_busy {
        receiver
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
    }
    router.view()
}

fn enter(router: &AppRouter) -> ViewModel {
    dispatch(router, AppEvent::SelectDestination(Destination::Vocabulary));
    dispatch(router, AppEvent::LoadVocabulary)
}

fn index(fixture: &Fixture) {
    let index = Connection::open(fixture.index()).unwrap();
    for (id, surface) in [(2, "食べる"), (3, "東京都"), (4, "東京"), (5, "東")] {
        let payload = format!(
            r#"{{"kanji":[{{"text":"{surface}","ke_pri":["ichi1"]}}],"readings":[{{"text":"とう","re_pri":["ichi1"]}}],"senses":[{{"pos":["noun (common) (futsuumeishi)"],"glosses":[{{"text":"test definition","lang":"eng"}}]}}]}}"#
        );
        index
            .execute(
                "INSERT INTO entries VALUES ('JMdict', ?1, ?2)",
                (id, payload),
            )
            .unwrap();
        index
            .execute("INSERT INTO forms VALUES (?1, 'JMdict', ?2)", (surface, id))
            .unwrap();
    }
    index.execute("INSERT INTO entries VALUES ('JMnedict', 1, ?1)", [r#"{"kanji":[{"text":"東京都"}],"readings":[{"text":"とうきょうと"}],"names":[{"name_type":["place name"],"translations":[{"text":"Tokyo Metropolis","lang":"eng"}]}]}"#]).unwrap();
    index
        .execute("INSERT INTO forms VALUES ('東京都', 'JMnedict', 1)", [])
        .unwrap();
}

#[test]
fn independent_lookup_keeps_lifetime_counts_after_book_deletion_and_never_writes() {
    let fixture = Fixture::new();
    index(&fixture);
    let mut storage = fixture.storage();
    let snapshot = storage
        .load_document_reader_snapshot(fixture.base.document_id)
        .unwrap()
        .unwrap();
    let page = ReaderPage::open_document(&snapshot.document, snapshot.page_length).unwrap();
    let command = storage_fixture::prepare(
        &mut storage,
        PageCompletionIntent {
            document_id: fixture.base.document_id,
            page: page.anchor,
            page_length: snapshot.page_length,
            departing_position: page.position,
            completed_at_unix_ms: 200,
            analysis_id: fixture.imported.analysis_id,
        },
    )
    .unwrap();
    storage.record_effective_page_completion(&command).unwrap();
    storage.delete_document(fixture.base.document_id).unwrap();
    storage
        .register_vocabulary(&[Surface::new("東京都").unwrap()], 300)
        .unwrap();
    let router = router(
        &fixture,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    );
    let rows = enter(&router).vocabulary.rows;
    let before = std::fs::read(fixture.path()).unwrap();
    for (surface, count, ids) in [
        ("食べた", 2, vec![2]),
        ("東京都", 0, vec![3]),
        ("𠮷か\u{3099}\0", 1, vec![]),
    ] {
        let row = rows.iter().find(|row| row.surface == surface).unwrap();
        let word = dispatch(&router, AppEvent::OpenVocabularyWord(row.id))
            .vocabulary
            .word
            .unwrap();
        assert_eq!(word.row.encountered_count, count);
        assert_eq!(word.row, *row);
        let candidates = word.candidates.unwrap();
        assert_eq!(
            candidates
                .words
                .iter()
                .map(|word| word.entry_id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(candidates.names.len(), usize::from(surface == "東京都"));
        if surface == "食べた" {
            assert_eq!(candidates.dictionary_form.as_deref(), Some("食べる"));
        }
        if surface == "東京都" {
            assert_eq!(candidates.names[0].entry.names[0].name_type, ["place name"]);
            assert_eq!(
                word.lookup
                    .parts
                    .iter()
                    .map(|part| part.surface.as_str())
                    .collect::<Vec<_>>(),
                ["東京", "東"]
            );
            assert_eq!(
                candidates.words[0].entry.senses[0].glosses[0].text,
                "test definition"
            );
        }
        dispatch(&router, AppEvent::DismissVocabularyWord);
        assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    }
    assert_eq!(
        storage
            .vocabulary_page(&VocabularyQuery::default(), 0)
            .unwrap()
            .rows,
        rows
    );
}

#[test]
fn analyzer_and_dictionary_failures_wait_for_explicit_retry() {
    let fixture = Fixture::new();
    let analyzer_failure = Arc::new(AtomicUsize::new(1));
    let dictionary_failure = Arc::new(AtomicUsize::new(1));
    let router = router(
        &fixture,
        analyzer_failure.clone(),
        dictionary_failure.clone(),
    );
    let row = enter(&router)
        .vocabulary
        .rows
        .into_iter()
        .find(|row| row.surface == "猫")
        .unwrap();
    let failed = dispatch(&router, AppEvent::OpenVocabularyWord(row.id));
    assert_eq!(
        failed.vocabulary.word.unwrap().error,
        Some(DocumentError::Unavailable)
    );
    analyzer_failure.store(0, Ordering::Relaxed);
    let failed = dispatch(&router, AppEvent::RetryVocabularyWord);
    assert!(failed.vocabulary.word.unwrap().error.is_some());
    dictionary_failure.store(0, Ordering::Relaxed);
    let unchanged = dispatch(&router, AppEvent::LoadVocabulary);
    assert!(unchanged.vocabulary.word.unwrap().error.is_some());
    let loaded = dispatch(&router, AppEvent::RetryVocabularyWord)
        .vocabulary
        .word
        .unwrap();
    assert!(loaded.error.is_none());
    assert_eq!(loaded.candidates.unwrap().words.len(), 1);
}

#[test]
fn direct_status_failure_retries_original_command_after_dismissal_and_refreshes_caches() {
    let fixture = Fixture::new();
    let router = router(
        &fixture,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0)),
    );
    dispatch(
        &router,
        AppEvent::SelectDestination(Destination::Statistics),
    );
    dispatch(
        &router,
        AppEvent::LoadStatistics {
            as_of_unix_ms: 1000,
        },
    );
    dispatch(&router, AppEvent::SelectDestination(Destination::Library));
    dispatch(&router, AppEvent::LoadLibraryWordDistributions);
    let opened = dispatch(&router, AppEvent::OpenReader(fixture.base.document_id));
    dispatch(
        &router,
        AppEvent::LoadReaderTokens(ReaderTokenKey {
            document_id: fixture.base.document_id,
            page: opened.reader.unwrap().anchor,
            analysis: opened.reader_analysis.unwrap(),
        }),
    );
    dispatch(&router, AppEvent::CloseDocument);
    let row = enter(&router)
        .vocabulary
        .rows
        .into_iter()
        .find(|row| row.surface == "猫")
        .unwrap();
    dispatch(&router, AppEvent::OpenVocabularyWord(row.id));
    fixture.sql().execute_batch("CREATE TRIGGER fail_word BEFORE UPDATE ON vocabulary BEGIN SELECT RAISE(ABORT, 'word status'); END;").unwrap();
    let command = VocabularyWordStatusCommand {
        id: row.id,
        status: VocabularyStatus::Learning,
        occurred_at_unix_ms: 555,
    };
    let failed = dispatch(&router, AppEvent::SetVocabularyWordStatus(command.clone()));
    assert_eq!(failed.vocabulary.word_status, Some(command.clone()));
    assert!(failed.vocabulary.word_status_error.is_some());
    assert_eq!(
        fixture
            .storage()
            .vocabulary_row(row.id)
            .unwrap()
            .unwrap()
            .status,
        row.status
    );
    fixture
        .sql()
        .execute_batch("DROP TRIGGER fail_word")
        .unwrap();
    dispatch(&router, AppEvent::DismissVocabularyWord);
    let changed = dispatch(&router, AppEvent::RetryVocabularyWordStatus);
    assert!(changed.vocabulary.word.is_none() && changed.vocabulary.word_status.is_none());
    assert!(changed.statistics.data.is_none() && changed.library_word_distributions.is_none());
    assert!(changed.effective_reader_tokens.is_none());
    let refreshed = dispatch(&router, AppEvent::LoadVocabulary);
    assert_eq!(
        refreshed
            .vocabulary
            .rows
            .iter()
            .find(|r| r.id == row.id)
            .unwrap()
            .status,
        VocabularyStatus::Learning
    );
    assert_eq!(fixture.sql().query_row("SELECT occurred_at_unix_ms FROM vocabulary_status_events WHERE cause = 'manual' AND vocabulary_id = ?", [i64::from(row.id)], |r| r.get::<_, i64>(0)).unwrap(), 555);
    drop(router);
    assert_eq!(
        fixture
            .storage()
            .vocabulary_row(row.id)
            .unwrap()
            .unwrap()
            .status,
        VocabularyStatus::Learning
    );
}
