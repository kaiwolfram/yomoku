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

#[test]
fn statistics_preference_public_events_round_trip_and_native_effects_stay_private() {
    for event in [
        Event::LoadStatisticsTimeframe,
        Event::SetAnkiDefaultDeck(Some(7)),
        Event::RetryAnkiSettings,
        Event::DismissAnkiAddedNotice(u64::MAX),
    ] {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &event).unwrap();
        assert_eq!(
            BincodeFfiFormat::deserialize::<Event>(&bytes).unwrap(),
            event
        );
    }
    for operation in [
        StorageOperation::LoadStatisticsTimeframe,
        StorageOperation::SaveStatisticsTimeframe(StatisticsTimeframe::LastWeek),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &operation).is_err());
    }
    for result in [
        StorageResult::StatisticsTimeframeLoaded(StatisticsTimeframe::LastWeek),
        StorageResult::StatisticsTimeframeSaved(StatisticsTimeframe::AllTime),
    ] {
        assert!(BincodeFfiFormat::serialize(&mut vec![], &result).is_err());
    }
    assert!(BincodeFfiFormat::serialize(
        &mut vec![],
        &Event::StatisticsTimeframeFinished(
            1,
            StorageResult::StatisticsTimeframeLoaded(StatisticsTimeframe::LastWeek)
        )
    )
    .is_err());
}
