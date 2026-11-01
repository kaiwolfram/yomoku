use std::{error::Error, fs, path::Path};

use yomoku_core::{
    anki::{
        AnkiExportError, AnkiLiveOperation, AnkiLiveResult, AnkiModelCreation, AnkiModelLayout,
        AnkiNote, AnkiPreflightDeck, AnkiPreflightModel, AnkiPreflightObservation,
        AnkiPreflightOperation, AnkiPreflightResult, AnkiReadModelLayout, AnkiSubmission,
    },
    document::{TextOffset, TextRange},
    vocabulary::Surface,
};
use yomoku_storage::Storage;

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* frozen 𠮷 */");
    // Exercise both directions of the shell protocol with Rust-owned comparison
    // data, Unicode and signed-64 IDs. Native preparation remains unexposed.
    let operation = AnkiPreflightOperation {
        surface: "𠮷猫".into(),
        deck_id: i64::MAX,
        layout: Box::new((&layout).into()),
    };
    let results = [
        AnkiPreflightResult::Complete(AnkiPreflightObservation {
            decks: vec![AnkiPreflightDeck {
                id: i64::MAX,
                filtered: false,
            }],
            duplicate_note_ids: vec![i64::MAX, 9007199254740993],
            models: vec![AnkiPreflightModel {
                id: 9007199254740993,
                name: layout.name.clone(),
                matches_layout: true,
            }],
        }),
        AnkiPreflightResult::Failed(AnkiExportError::PermissionDenied),
    ];
    for (name, bytes) in std::iter::once(("operation".to_owned(), super::encode(&operation))).chain(
        results
            .iter()
            .enumerate()
            .map(|(i, result)| (format!("result-{i}"), super::encode(result))),
    ) {
        if writing {
            fs::write(dir.join(format!("anki-preflight-{name}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-preflight-{name}-return.bin")))?,
                bytes
            );
        }
    }
    let operations = [
        AnkiLiveOperation::InsertNote {
            surface: "𠮷猫".into(),
            fields: vec![
                "𠮷猫".into(),
                "ねこ".into(),
                "<ol><li>cat</li></ol>".into(),
                "<strong>𠮷猫</strong>。".into(),
            ],
            deck_id: i64::MAX,
            model_id: 9007199254740993,
            layout: Box::new((&layout).into()),
        },
        AnkiLiveOperation::CreateModel {
            surface: "𠮷猫".into(),
            deck_id: i64::MAX,
            layout: Box::new((&layout).into()),
        },
    ];
    let live_results = [
        AnkiLiveResult::NoSubmission,
        AnkiLiveResult::NoteAdded(i64::MAX),
        AnkiLiveResult::NoteExisting(vec![i64::MAX, 9007199254740993]),
        AnkiLiveResult::NoteUncertain(None),
        AnkiLiveResult::NoteUncertain(Some(i64::MAX)),
        AnkiLiveResult::ModelCreated(9007199254740993),
        AnkiLiveResult::ModelUncertain(None),
        AnkiLiveResult::ModelUncertain(Some(i64::MAX)),
    ];
    for (name, bytes) in operations
        .iter()
        .enumerate()
        .map(|(i, op)| (format!("operation-{i}"), super::encode(op)))
        .chain(
            live_results
                .iter()
                .enumerate()
                .map(|(i, result)| (format!("result-{i}"), super::encode(result))),
        )
    {
        if writing {
            fs::write(dir.join(format!("anki-live-{name}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-live-{name}-return.bin")))?,
                bytes
            );
        }
    }
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
