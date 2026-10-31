use std::{error::Error, fs, path::Path};

use yomoku_core::{
    anki::{AnkiModelCreation, AnkiModelLayout, AnkiNote, AnkiReadModelLayout, AnkiSubmission},
    document::{TextOffset, TextRange},
    vocabulary::Surface,
};
use yomoku_storage::Storage;

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* frozen 𠮷 */");
    if writing {
        fs::write(
            dir.join("anki-layout.bin"),
            super::encode(&AnkiReadModelLayout::from(&layout)),
        )?;
        for model in [false, true] {
            let name = if model { "model" } else { "note" };
            let path = dir.join(format!("anki-{name}-seed.sqlite3"));
            if path.exists() {
                fs::remove_file(&path)?;
            }
            let mut storage = Storage::open(path)?;
            if model {
                storage.begin_anki_model_creation(&AnkiModelCreation::new(layout.clone())?)?;
            } else {
                let note = AnkiNote::new(
                    &Surface::new("𠮷猫")?,
                    "ねこ",
                    &["cat"],
                    "𠮷猫。",
                    TextRange {
                        start: TextOffset::START,
                        end: 7.try_into()?,
                    },
                )?;
                storage.begin_anki_submission(&AnkiSubmission::new(
                    note,
                    9007199254740993_i64.try_into()?,
                    i64::MAX.try_into()?,
                    layout.clone(),
                )?)?;
            }
        }
    } else {
        assert_eq!(
            fs::read(dir.join("anki-layout-return.bin"))?,
            super::encode(&AnkiReadModelLayout::from(&layout))
        );
        for model in [false, true] {
            let name = if model { "model" } else { "note" };
            let mut storage = Storage::open(dir.join(format!("anki-{name}-jni.sqlite3")))?;
            assert!(storage.anki_recovery_snapshot()?.note.is_none());
            assert!(storage.anki_recovery_snapshot()?.model.is_none());
        }
    }
    Ok(())
}
