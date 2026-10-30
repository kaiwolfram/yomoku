use crux_core::bridge::{BincodeFfiFormat, FfiFormat};
use yomoku_core::{
    app::Event,
    statistics::{statistics_buckets, StatisticsQuery, StatisticsTimeframe},
    storage::{StorageOperation, StorageResult},
};

#[test]
fn statistics_worker_requests_results_and_callbacks_have_no_shell_codec() {
    let query = StatisticsQuery {
        timeframe: StatisticsTimeframe::LastYear,
        as_of_unix_ms: 1_800_000_000_123,
    };
    assert!(
        BincodeFfiFormat::serialize(&mut vec![], &StorageOperation::LoadStatistics(query)).is_err()
    );
    let data = statistics_buckets(query.timeframe, query.as_of_unix_ms, &[], &[]).unwrap();
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &StorageResult::StatisticsLoaded(Box::new(data))
    )
    .is_err());
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &Event::StatisticsFinished(
            1,
            StorageResult::Failed(yomoku_core::storage::DocumentError::Unavailable)
        )
    )
    .is_err());
}
