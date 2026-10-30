use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    app::Event,
    storage::{DocumentError, StorageOperation, StorageResult},
    vocabulary::{VocabularyBulkAction, VocabularyBulkCommand, VocabularyPage, VocabularyQuery},
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

#[test]
fn vocabulary_bulk_worker_payloads_and_captured_callbacks_have_no_shell_codec() {
    let command = VocabularyBulkCommand {
        ids: vec![i64::MAX.try_into().unwrap()],
        action: VocabularyBulkAction::Delete,
    };
    for operation in [
        StorageOperation::VocabularyMatchingIds(VocabularyQuery::default()),
        StorageOperation::ApplyVocabularyBulk(command.clone()),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    }
    for result in [
        StorageResult::VocabularyMatchingIds(command.ids.clone()),
        StorageResult::VocabularyBulkApplied(command),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
        assert!(BincodeFfiFormat::serialize(
            &mut vec![],
            &Event::VocabularyBulkFinished(1, result)
        )
        .is_err());
    }
}
