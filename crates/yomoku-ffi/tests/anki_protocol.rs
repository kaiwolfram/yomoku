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
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &event).is_err());
    }
}
