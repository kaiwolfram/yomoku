use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use std::{error::Error, fs, path::Path};
use yomoku_core::{
    anki::{AnkiSettingsPhase, AnkiSettingsView},
    app::Event,
    storage::DocumentError,
};

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    for (index, event) in [
        Event::LoadAnkiSettings,
        Event::RetryAnkiSettings,
        Event::SetAnkiDefaultDeck(None),
        Event::SetAnkiDefaultDeck(Some(9007199254740993)),
        Event::SetAnkiDefaultDeck(Some(i64::MAX)),
        Event::SetAnkiDefaultDeck(Some(0)),
        Event::SetAnkiDefaultDeck(Some(i64::MIN)),
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = super::encode(&event);
        assert_eq!(BincodeFfiFormat::deserialize::<Event>(&bytes)?, event);
        if writing {
            fs::write(dir.join(format!("anki-settings-action-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-settings-action-{index}-return.bin")))?,
                bytes
            );
        }
    }
    for (index, phase) in [
        AnkiSettingsPhase::Unloaded,
        AnkiSettingsPhase::Loading,
        AnkiSettingsPhase::Ready,
        AnkiSettingsPhase::Saving,
        AnkiSettingsPhase::Failed,
    ]
    .into_iter()
    .enumerate()
    {
        let view = AnkiSettingsView {
            phase,
            default_deck_id: if index < 2 { None } else { Some(i64::MAX) },
            storage_error: (phase == AnkiSettingsPhase::Failed)
                .then_some(DocumentError::InvalidSettings),
        };
        let bytes = super::encode(&view);
        if writing {
            fs::write(dir.join(format!("anki-settings-view-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                BincodeFfiFormat::deserialize::<AnkiSettingsView>(&fs::read(
                    dir.join(format!("anki-settings-view-{index}-return.bin"))
                )?)?,
                view
            );
        }
    }
    Ok(())
}
