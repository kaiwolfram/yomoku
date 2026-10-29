use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    document::{TextAnchor, TextRange},
    effective_reader::EffectiveReaderTokenSnapshot,
    effective_session::EffectiveReaderSession,
    effective_storage::{EffectiveStorageOperation, EffectiveStorageResult},
    reader::{ReaderAnalysis, ReaderTokenKey},
    storage::{StorageOperation, StorageResult},
};

#[test]
fn effective_worker_protocol_has_no_shell_codec() {
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &StorageOperation::LoadEffectiveLibraryWordDistributions
    )
    .is_err());
    let base = ReaderTokenKey {
        document_id: 1.try_into().unwrap(),
        page: TextAnchor {
            source_id: 2.try_into().unwrap(),
            chapter_id: 3.try_into().unwrap(),
            range: TextRange {
                start: 0.try_into().unwrap(),
                end: 3.try_into().unwrap(),
            },
        },
        analysis: ReaderAnalysis::Missing,
    };
    let mut session = EffectiveReaderSession::default();
    let request = session.begin_load(base).unwrap();
    let operation = StorageOperation::Effective(EffectiveStorageOperation::LoadTokens(request));
    assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    let result = StorageResult::Effective(EffectiveStorageResult::TokensLoaded(
        request,
        EffectiveReaderTokenSnapshot {
            document_id: base.document_id,
            page: base.page,
            segmentation: None,
            tokens: vec![],
        },
    ));
    assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
}
