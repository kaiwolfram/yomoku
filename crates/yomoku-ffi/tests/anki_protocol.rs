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
        Event::AnkiRecoveryFinished(1, result),
        Event::AnkiRecoveryFinished(2, StorageResult::Failed(DocumentError::Unavailable)),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}

#[test]
fn anki_reconciliation_commits_and_results_cannot_cross_shell_codecs() {
    use yomoku_core::anki::{AnkiModelReconciliation, AnkiRecoveryCommit, AnkiRecoveryObservation};
    let command = Box::new(AnkiRecoveryCommit {
        expected: Box::default(),
        observation: AnkiRecoveryObservation::Model(AnkiModelReconciliation::Unresolved),
    });
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &StorageOperation::CommitAnkiRecovery(command.clone())
    )
    .is_err());
    let result = StorageResult::AnkiRecoveryCommitted(Box::default());
    assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
    for event in [
        Event::CommitAnkiRecovery(command),
        Event::AnkiRecoveryCommitFinished(1, result),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}

#[test]
fn anki_export_preparation_and_clear_have_no_shell_codec() {
    use yomoku_core::{
        anki::AnkiSelection,
        document::{TextAnchor, TextOffset, TextRange},
        reader::{ReaderAnalysis, ReaderTokenKey, ReaderWordSelection},
    };
    let selection = AnkiSelection::Original(ReaderWordSelection {
        key: ReaderTokenKey {
            document_id: 1.try_into().unwrap(),
            page: TextAnchor {
                source_id: 2.try_into().unwrap(),
                chapter_id: 3.try_into().unwrap(),
                range: TextRange {
                    start: TextOffset::START,
                    end: 3.try_into().unwrap(),
                },
            },
            analysis: ReaderAnalysis::Available(4.try_into().unwrap()),
        },
        token_index: 0,
    });
    for event in [
        Event::PrepareAnkiExport {
            selection,
            deck: i64::MAX.try_into().unwrap(),
        },
        Event::ClearPreparedAnkiExport,
        Event::BeginAnkiLive,
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}

#[test]
fn anki_recovery_read_dispatch_is_public_but_captured_callback_has_no_shell_codec() {
    use crux_core::App;
    use yomoku_core::{
        anki::{
            AnkiModelCreation, AnkiModelLayout, AnkiModelReconciliation, AnkiModelRecovery,
            AnkiRecoveryObservation, AnkiRecoveryReadResult, AnkiRecoveryReadTarget,
            AnkiRecoverySnapshot,
        },
        app::{Model, Yomoku},
    };
    for target in [AnkiRecoveryReadTarget::Note, AnkiRecoveryReadTarget::Model] {
        assert!(
            BincodeFfiFormat::serialize(&mut vec![], &Event::BeginAnkiRecoveryRead(target)).is_ok()
        );
    }
    assert!(BincodeFfiFormat::serialize(&mut vec![], &Event::RetryAnkiRecovery).is_ok());
    let mut model = Model::default();
    let _restore = Yomoku.update(Event::RestoreAnkiRecovery, &mut model);
    let _finished = Yomoku.update(
        Event::AnkiRecoveryFinished(
            1,
            StorageResult::AnkiRecoveryLoaded(Box::new(AnkiRecoverySnapshot {
                note: None,
                model: Some(AnkiModelRecovery {
                    id: 1.try_into().unwrap(),
                    creation: AnkiModelCreation::new(AnkiModelLayout::yomoku()).unwrap(),
                    known_model_id: None,
                }),
            })),
        ),
        &mut model,
    );
    let _begin = Yomoku.update(
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
        &mut model,
    );
    let event = Event::AnkiRecoveryReadFinished(
        Box::new(model.pending_anki_recovery_read().unwrap().clone()),
        Box::new(AnkiRecoveryObservation::Model(
            AnkiModelReconciliation::Unresolved,
        )),
    );
    assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    let event = Event::AnkiRecoveryProviderFinished(
        Box::new(model.pending_anki_recovery_read().unwrap().clone()),
        AnkiRecoveryReadResult::Model(Some(i64::MAX)),
    );
    assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
}

#[test]
fn anki_public_read_events_round_trip_after_skipped_native_callbacks() {
    use yomoku_core::anki::AnkiRecoveryReadTarget;
    for event in [
        Event::DispatchAnkiLive,
        Event::BeginAnkiExportPreflight,
        Event::RetryAnkiExportPreflight,
        Event::RetryAnkiRecovery,
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Note),
        Event::BeginAnkiRecoveryRead(AnkiRecoveryReadTarget::Model),
    ] {
        let mut bytes = vec![];
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        assert_eq!(
            BincodeFfiFormat::deserialize::<Event>(&bytes).unwrap(),
            event
        );
    }
}
