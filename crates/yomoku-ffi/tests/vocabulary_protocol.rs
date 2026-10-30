use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    app::Event,
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{VocabularyPage, VocabularyQuery},
};

#[test]
fn vocabulary_worker_requests_results_and_callbacks_have_no_shell_codec() {
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &StorageOperation::LoadVocabulary {
            query: VocabularyQuery::default(),
            offset: 0,
        }
    )
    .is_err());
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &StorageResult::VocabularyLoaded(VocabularyPage {
            matching_count: 0,
            offset: 0,
            rows: vec![],
            next_offset: None,
        })
    )
    .is_err());
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &Event::VocabularyFinished(1, StorageResult::Failed(DocumentError::Unavailable),)
    )
    .is_err());
}
