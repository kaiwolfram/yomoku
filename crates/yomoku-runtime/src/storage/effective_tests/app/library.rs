use super::*;
use yomoku_core::library::WordStatusCounts;

fn read(router: &AppRouter, fixture: &Fixture) -> ViewModel {
    let before = std::fs::read(fixture.path()).unwrap();
    let view = dispatch(router, AppEvent::LoadLibraryWordDistributions);
    assert_eq!(std::fs::read(fixture.path()).unwrap(), before);
    assert!(!view.storage_busy);
    view
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
