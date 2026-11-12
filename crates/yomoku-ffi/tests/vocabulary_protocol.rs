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
        action: VocabularyBulkAction::SetStatus {
            status: yomoku_core::vocabulary::VocabularyStatus::Known,
            occurred_at_unix_ms: 999,
        },
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

#[test]
fn vocabulary_word_operations_and_callbacks_are_native_only_and_deletion_is_unrepresentable() {
    use yomoku_core::vocabulary::{VocabularyWordDetails, VocabularyWordStatusCommand};
    let command = VocabularyWordStatusCommand {
        id: 1.try_into().unwrap(),
        status: yomoku_core::vocabulary::VocabularyStatus::Learning,
        occurred_at_unix_ms: 123,
    };
    for operation in [
        StorageOperation::LoadVocabularyWord(command.id),
        StorageOperation::SetVocabularyWordStatus(command.clone()),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    }
    let result = StorageResult::VocabularyWordLoaded(Box::new(VocabularyWordDetails {
        row: yomoku_core::vocabulary::VocabularyRow {
            id: command.id,
            surface: "猫".into(),
            status: command.status,
            registered_at_unix_ms: 123,
            encountered_count: 0,
            last_seen_unix_ms: None,
        },
        candidates: yomoku_core::dictionary::DictionaryCandidates {
            surface: "猫".into(),
            dictionary_form: None,
            words: vec![],
            names: vec![],
        },
    }));
    for result in [result, StorageResult::VocabularyWordStatusChanged(command)] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
        assert!(BincodeFfiFormat::serialize(
            &mut vec![],
            &Event::VocabularyWordFinished(1, result)
        )
        .is_err());
    }
    // The historical Delete discriminant has no supported replacement action.
    assert!(BincodeFfiFormat::deserialize::<VocabularyBulkAction>(&1_u32.to_le_bytes()).is_err());
}
