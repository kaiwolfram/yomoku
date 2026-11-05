use std::{error::Error, fs, path::Path};

use crux_core::{
    bridge::{BincodeFfiFormat, FfiFormat},
    Core,
};
use yomoku_core::{
    app::{Event, ViewModel, Yomoku},
    library::WordStatusCounts,
    statistics::{
        LifetimeReadingTotals, StatisticsBucket, StatisticsBuckets, StatisticsQuery,
        StatisticsState, StatisticsTimeframe, StatisticsValues,
    },
    storage::DocumentError,
};

pub fn exchange(dir: &Path, writing: bool) -> Result<(), Box<dyn Error>> {
    if writing {
        fs::create_dir_all(dir)?;
    }
    for (index, timeframe) in [
        StatisticsTimeframe::LastMonth,
        StatisticsTimeframe::LastSixMonths,
        StatisticsTimeframe::LastYear,
        StatisticsTimeframe::AllTime,
    ]
    .into_iter()
    .enumerate()
    {
        let query = StatisticsQuery {
            timeframe,
            as_of_unix_ms: 1_800_000_000_123,
        };
        let values = StatisticsValues {
            reading: LifetimeReadingTotals {
                japanese_occurrences: u64::MAX,
            },
            statuses: WordStatusCounts {
                new: 1,
                learning: 2,
                familiar: 3,
                known: (1 << 53) + 1,
                ignored: 5,
            },
        };
        let mut view = Core::<Yomoku>::new().view();
        view.statistics = StatisticsState {
            timeframe,
            query: Some(query),
            loading: false,
            error: None,
            data: Some(StatisticsBuckets {
                timeframe,
                start_unix_ms: 1_799_000_000_000,
                end_exclusive_unix_ms: query.as_of_unix_ms + 1,
                opening: values,
                buckets: vec![StatisticsBucket {
                    start_unix_ms: 1_799_000_000_000,
                    end_exclusive_unix_ms: query.as_of_unix_ms + 1,
                    values,
                }],
            }),
        };
        let event = Event::SetStatisticsTimeframe {
            timeframe,
            as_of_unix_ms: query.as_of_unix_ms,
        };
        if writing {
            fs::write(
                dir.join(format!("statistics-view-{index}.bin")),
                super::encode(&view),
            )?;
            fs::write(
                dir.join(format!("statistics-event-{index}.bin")),
                super::encode(&event),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("statistics-view-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
            let returned: Event = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("statistics-event-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, event);
        }
    }
    // Error/loading/absent projections also pass through the generated view codec.
    for (index, state) in [
        StatisticsState::default(),
        StatisticsState {
            loading: true,
            ..Default::default()
        },
        StatisticsState {
            error: Some(DocumentError::InvalidDocument),
            ..Default::default()
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = Core::<Yomoku>::new().view();
        view.statistics = state;
        if writing {
            fs::write(
                dir.join(format!("statistics-state-{index}.bin")),
                super::encode(&view),
            )?;
        } else {
            let returned: ViewModel = BincodeFfiFormat::deserialize(&fs::read(
                dir.join(format!("statistics-state-{index}-return.bin")),
            )?)?;
            assert_eq!(returned, view);
        }
    }
    Ok(())
}
