use super::*;
use yomoku_core::library::{LibraryWordDistribution, WordStatusCounts};

fn counts(fixture: &Fixture, counts: WordStatusCounts) -> Option<Vec<LibraryWordDistribution>> {
    Some(vec![LibraryWordDistribution {
        document_id: fixture.base.document_id,
        counts: Some(counts),
    }])
}

fn read(router: &AppRouter, fixture: &Fixture) -> ViewModel {
    let before = std::fs::read(fixture.path()).unwrap();
    let view = dispatch(router, AppEvent::LoadLibraryWordDistributions);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert!(!view.storage_busy);
    view
}

#[test]
fn effective_application_fifo_library_membership_status_refresh_reversal_and_reopen() {
    let fixture = Fixture::new();
    let router = router(&fixture); // Both dictionary loaders panic if invoked.
    dispatch(&router, AppEvent::RefreshLibrary);
    let view = read(&router, &fixture);
    // Original B has three surfaces. Effective split parts are still unregistered,
    // so only the other two surfaces appear. Repetitions never inflate counts.
    assert_eq!(
        view.library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 2,
                ..Default::default()
            }
        )
    );
    assert_eq!(
        fixture.storage().library_word_distributions().unwrap()[0]
            .counts
            .unwrap()
            .new,
        3
    );

    let mut storage = fixture.storage();
    storage
        .register_vocabulary(
            &[Surface::new("食べ").unwrap(), Surface::new("た").unwrap()],
            200,
        )
        .unwrap();
    let entry = storage
        .vocabulary_entry(&Surface::new("食べ").unwrap())
        .unwrap()
        .unwrap();
    storage
        .set_vocabulary_status(entry.id, VocabularyStatus::Known, 201)
        .unwrap();
    // Retained presentation is stable until an explicit refresh/return.
    assert_eq!(
        router.view().library_word_distributions,
        view.library_word_distributions
    );
    dispatch(&router, AppEvent::RefreshLibrary);
    assert!(router.view().library_word_distributions.is_none());
    let view = read(&router, &fixture);
    assert_eq!(
        view.library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 3,
                known: 1,
                ..Default::default()
            }
        )
    );
    fixture
        .sql()
        .execute("DELETE FROM vocabulary WHERE surface = '猫'", [])
        .unwrap();
    let view = read(&router, &fixture);
    assert_eq!(
        view.library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 2,
                known: 1,
                ..Default::default()
            }
        )
    );

    storage.reverse_segmentation_override(fixture.rule).unwrap();
    let view = read(&router, &fixture);
    assert_eq!(
        view.library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 2,
                ..Default::default()
            }
        )
    );
    let fresh = super::router(&fixture);
    assert_eq!(
        read(&fresh, &fixture).library_word_distributions,
        view.library_word_distributions
    );

    // The production constructor still returns B even when the split is active.
    fixture
        .sql()
        .execute(
            "UPDATE segmentation_overrides SET active = 1 WHERE id = ?1",
            [i64::from(fixture.rule)],
        )
        .unwrap();
    let effects = StorageEffects::new(fixture.path()).unwrap();
    let original = Core::<Yomoku>::new().handle_effects_using(effects);
    assert_eq!(
        read(&original, &fixture).library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 2,
                ..Default::default()
            }
        )
    );
    assert_eq!(
        read(&router, &fixture).library_word_distributions,
        counts(
            &fixture,
            WordStatusCounts {
                new: 2,
                known: 1,
                ..Default::default()
            }
        )
    );
    let sql = fixture.sql();
    for table in [
        "reading_completions",
        "reading_occurrences",
        "effective_completion_commands",
    ] {
        assert_eq!(
            sql.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[test]
fn effective_application_fifo_library_missing_zero_corrupt_explicit_retry_and_deleted_document() {
    let fixture = Fixture::new();
    let missing = fixture
        .storage()
        .create_paste(&PasteText::new("未解析".into()).unwrap())
        .unwrap();
    let zero = fixture
        .storage()
        .create_analyzed_paste(&PasteText::new("English 。".into()).unwrap(), 200, analysis)
        .unwrap();
    let router = router(&fixture);
    let view = read(&router, &fixture);
    let distributions = view.library_word_distributions.unwrap();
    assert_eq!(
        distributions
            .iter()
            .find(|d| d.document_id == missing.document_id)
            .unwrap()
            .counts,
        None
    );
    assert_eq!(
        distributions
            .iter()
            .find(|d| d.document_id == zero.document.document_id)
            .unwrap()
            .counts,
        Some(WordStatusCounts::default())
    );

    // Fail whole-chapter validation without publishing a partial distribution.
    fixture.sql().execute("UPDATE analyzed_tokens SET surface = '壊れた' WHERE analysis_id = ?1 AND token_index = 0", [i64::from(fixture.imported.analysis_id)]).unwrap();
    let failed = read(&router, &fixture);
    assert!(failed.library_word_distributions.is_none());
    assert!(failed.library_word_error.is_some());
    assert!(failed.document_error.is_none());
    fixture.sql().execute("UPDATE analyzed_tokens SET surface = '食べた' WHERE analysis_id = ?1 AND token_index = 0", [i64::from(fixture.imported.analysis_id)]).unwrap();
    // Repair alone never retries. Presentation-only events preserve the error.
    let view = dispatch(&router, AppEvent::SearchLibraryTitles("本".into()));
    assert_eq!(view.library_word_error, failed.library_word_error);
    let view = read(&router, &fixture);
    assert!(view.library_word_error.is_none());
    assert_eq!(view.library_word_distributions.unwrap().len(), 3);

    fixture
        .storage()
        .delete_document(fixture.base.document_id)
        .unwrap();
    let view = read(&router, &fixture);
    let distributions = view.library_word_distributions.unwrap();
    assert_eq!(distributions.len(), 2);
    assert!(distributions
        .iter()
        .all(|d| d.document_id != fixture.base.document_id));
}
