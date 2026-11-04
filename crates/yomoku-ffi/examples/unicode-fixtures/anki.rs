use crux_core::bridge::FfiFormat;
use std::{error::Error, fs, path::Path};

use yomoku_core::{
    anki::{
        AnkiExportError, AnkiExportPhase, AnkiExportView, AnkiLiveOperation, AnkiLiveResult,
        AnkiModelLayout, AnkiPreflightDeck, AnkiPreflightModel, AnkiPreflightObservation,
        AnkiPreflightOperation, AnkiPreflightResult, AnkiReadModelLayout, AnkiSelection,
    },
    app::Event,
    document::{TextAnchor, TextOffset, TextRange},
    effective_session::{EffectivePageIdentity, EffectiveReaderSelection},
    reader::{ReaderAnalysis, ReaderTokenKey, ReaderWordSelection},
};

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dir)?;
    let key = ReaderTokenKey {
        document_id: i64::MAX.try_into()?,
        page: TextAnchor {
            source_id: 9007199254740993.try_into()?,
            chapter_id: i64::MAX.try_into()?,
            range: TextRange {
                start: TextOffset::START,
                end: 3.try_into()?,
            },
        },
        analysis: ReaderAnalysis::Available(i64::MAX.try_into()?),
    };
    let entry_selection = AnkiSelection::Original(ReaderWordSelection {
        key,
        token_index: u64::MAX,
    });
    for (index, event) in [
        Event::CheckAnkiSetup,
        Event::CreateAnkiModel,
        Event::LoadReaderAnkiEntries(entry_selection),
        Event::AddReaderAnkiEntry {
            selection: entry_selection,
            entry: yomoku_core::anki::AnkiEntryKey::Word(i64::MAX),
        },
        Event::AddReaderAnkiEntry {
            selection: entry_selection,
            entry: yomoku_core::anki::AnkiEntryKey::Name(9007199254740993),
        },
        Event::AddReaderAnkiEntry {
            selection: entry_selection,
            entry: yomoku_core::anki::AnkiEntryKey::Word(1),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = super::encode(&event);
        if writing {
            fs::write(dir.join(format!("anki-entry-action-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-entry-action-{index}-return.bin")))?,
                bytes
            );
        }
    }
    for (index, phase) in [
        yomoku_core::anki::AnkiEntryPhase::Checking,
        yomoku_core::anki::AnkiEntryPhase::Add,
        yomoku_core::anki::AnkiEntryPhase::Existing(vec![i64::MAX, 9007199254740993]),
        yomoku_core::anki::AnkiEntryPhase::Export(AnkiExportPhase::NoteUncertain(Some(i64::MAX))),
        yomoku_core::anki::AnkiEntryPhase::Failed(AnkiExportError::PermissionDenied),
    ]
    .into_iter()
    .enumerate()
    {
        let view = yomoku_core::anki::AnkiEntryView {
            selection: entry_selection,
            key: yomoku_core::anki::AnkiEntryKey::Word(i64::MAX),
            headword: "𠮷<&".into(),
            phase,
        };
        let bytes = super::encode(&view);
        if writing {
            fs::write(dir.join(format!("anki-entry-view-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-entry-view-{index}-return.bin")))?,
                bytes
            );
        }
    }
    for (index, event) in [
        Event::PrepareReaderAnkiExport {
            selection: AnkiSelection::Original(ReaderWordSelection {
                key,
                token_index: u64::MAX,
            }),
        },
        Event::PrepareReaderAnkiExport {
            selection: AnkiSelection::Effective(EffectiveReaderSelection {
                page: EffectivePageIdentity {
                    base: key,
                    revision: u64::MAX,
                },
                segment_index: u64::MAX,
            }),
        },
        Event::BeginPreparedAnkiExport {
            preparation_id: u64::MAX,
            expected_phase: AnkiExportPhase::CreateModel,
        },
        Event::BeginPreparedAnkiExport {
            preparation_id: u64::MAX,
            expected_phase: AnkiExportPhase::ReuseModel(9007199254740993),
        },
        Event::ClearAnkiExport {
            preparation_id: u64::MAX,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = super::encode(&event);
        assert_eq!(
            crux_core::bridge::BincodeFfiFormat::deserialize::<Event>(&bytes)?,
            event
        );
        if writing {
            fs::write(dir.join(format!("anki-action-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-action-{index}-return.bin")))?,
                bytes
            );
        }
    }
    for (index, phase) in [
        AnkiExportPhase::Prepared,
        AnkiExportPhase::Checking,
        AnkiExportPhase::CreateModel,
        AnkiExportPhase::ReuseModel(9007199254740993),
        AnkiExportPhase::Existing(vec![i64::MAX, 9007199254740993]),
        AnkiExportPhase::Ready,
        AnkiExportPhase::Invoking,
        AnkiExportPhase::NoteAdded(i64::MAX),
        AnkiExportPhase::ModelCreated(9007199254740993),
        AnkiExportPhase::NotSubmitted,
        AnkiExportPhase::NoteUncertain(None),
        AnkiExportPhase::NoteUncertain(Some(i64::MAX)),
        AnkiExportPhase::ModelUncertain(None),
        AnkiExportPhase::ModelUncertain(Some(9007199254740993)),
        AnkiExportPhase::ProviderFailed(AnkiExportError::IncompatibleModel),
    ]
    .into_iter()
    .enumerate()
    {
        let view = AnkiExportView {
            preparation_id: u64::MAX,
            surface: "𠮷猫".into(),
            deck_id: i64::MAX,
            phase,
        };
        let bytes = super::encode(&view);
        if writing {
            fs::write(dir.join(format!("anki-export-view-{index}.bin")), bytes)?;
        } else {
            assert_eq!(
                fs::read(dir.join(format!("anki-export-view-{index}-return.bin")))?,
                bytes
            );
        }
    }
    let mut layout = AnkiModelLayout::yomoku();
    layout.css.push_str("/* frozen 𠮷 */");
    // Exercise both directions of the shell protocol with Rust-owned comparison
    // data, Unicode and signed-64 IDs. Native callbacks remain unexposed.
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
    } else {
        assert_eq!(
            fs::read(dir.join("anki-layout-return.bin"))?,
            super::encode(&AnkiReadModelLayout::from(&layout))
        );
    }
    Ok(())
}
