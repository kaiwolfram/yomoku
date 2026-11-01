use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    app::Event,
    storage::{StorageOperation, StorageResult},
};

#[test]
fn anki_settings_native_storage_and_callback_have_no_shell_codec() {
    for operation in [
        StorageOperation::LoadAnkiSettings,
        StorageOperation::SaveAnkiDefaultDeck(Some(i64::MAX.try_into().unwrap())),
        StorageOperation::SaveAnkiDefaultDeck(None),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    }
    for result in [
        StorageResult::AnkiSettingsLoaded(None),
        StorageResult::AnkiDefaultDeckSaved(Some(i64::MAX.try_into().unwrap())),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
        assert!(
            BincodeFfiFormat::serialize(&mut vec![], &Event::AnkiSettingsFinished(1, result))
                .is_err()
        );
    }
}
