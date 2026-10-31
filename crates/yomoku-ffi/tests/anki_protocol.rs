use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    app::Event,
    storage::{DocumentError, StorageOperation, StorageResult},
};

#[test]
fn anki_startup_worker_payloads_and_callbacks_have_no_shell_codec() {
    assert!(BincodeFfiFormat::serialize(&mut vec![], &StorageOperation::LoadAnkiRecovery).is_err());
    let result = StorageResult::AnkiRecoveryLoaded(Box::default());
    assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
    for event in [
        Event::RestoreAnkiRecovery,
        Event::RetryAnkiRecovery,
        Event::AnkiRecoveryFinished(1, result),
        Event::AnkiRecoveryFinished(2, StorageResult::Failed(DocumentError::Unavailable)),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}
