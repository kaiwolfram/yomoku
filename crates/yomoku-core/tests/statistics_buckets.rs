use yomoku_core::{
    library::WordStatusCounts,
    statistics::{
        statistics_buckets, LifetimeReadingTotals, ReadingTimelinePoint,
        StatisticsTimeframe::{self, *},
        StatusTimelinePoint,
    },
};

const DAY: i64 = 86_400_000;

// Independent UTC fixtures, generated with Python datetime, not the projection's
// Gregorian conversion. Keeping them literal also tests the Unix epoch mapping.
fn date(iso: &str) -> i64 {
    match iso {
        "2023-03-31" => 1_680_220_800_000,
        "2023-09-30" => 1_696_032_000_000,
        "2024-02-29" => 1_709_164_800_000,
        "2024-03-01" => 1_709_251_200_000,
        "2024-03-04" => 1_709_510_400_000,
        "2024-03-30" => 1_711_756_800_000,
        "2024-03-31" => 1_711_843_200_000,
        "2024-07-03" => 1_719_964_800_000,
        "2024-07-08" => 1_720_396_800_000,
        "2024-12-30" => 1_735_516_800_000,
        "2024-12-31" => 1_735_603_200_000,
        "2025-01-01" => 1_735_689_600_000,
        "2025-01-03" => 1_735_862_400_000,
        "2023-02-28" => 1_677_542_400_000,
        "2000-02-29" => 951_782_400_000,
        "2000-03-31" => 954_460_800_000,
        "2100-02-28" => 4_107_456_000_000,
        "2100-03-31" => 4_110_134_400_000,
        "2024-01-01" => 1_704_067_200_000,
        "2024-02-01" => 1_706_745_600_000,
        "2024-04-01" => 1_711_929_600_000,
        _ => panic!("missing date fixture: {iso}"),
    }
}

fn reading(timestamp: i64, occurrences: u64) -> ReadingTimelinePoint {
    ReadingTimelinePoint {
        completed_at_unix_ms: timestamp,
        totals: LifetimeReadingTotals {
            japanese_occurrences: occurrences,
        },
    }
}

fn status(timestamp: i64, counts: [u64; 5]) -> StatusTimelinePoint {
    let [new, learning, familiar, known, ignored] = counts;
    StatusTimelinePoint {
        occurred_at_unix_ms: timestamp,
        counts: WordStatusCounts {
            new,
            learning,
            familiar,
            known,
            ignored,
        },
    }
}

#[test]
fn defaults_rolling_calendar_months_and_partial_today() {
    assert_eq!(StatisticsTimeframe::default(), LastYear);
    let now = date("2024-03-31") + DAY / 2;
    for (frame, start) in [
        (LastMonth, "2024-02-29"),
        (LastSixMonths, "2023-09-30"),
        (LastYear, "2023-03-31"),
        (AllTime, "2024-03-01"),
    ] {
        let result = statistics_buckets(frame, now, &[], &[]).unwrap();
        assert_eq!(result.timeframe, frame);
        assert_eq!(result.start_unix_ms, date(start));
        assert_eq!(result.end_exclusive_unix_ms, now + 1);
        assert_eq!(result.buckets.first().unwrap().start_unix_ms, date(start));
        assert_eq!(
            result.buckets.last().unwrap().end_exclusive_unix_ms,
            now + 1
        );
        assert!(result
            .buckets
            .iter()
            .all(|b| b.values == Default::default()));
        for pair in result.buckets.windows(2) {
            assert_eq!(pair[0].end_exclusive_unix_ms, pair[1].start_unix_ms);
        }
        if frame == LastMonth {
            assert_eq!(result.buckets.len(), 32);
            assert!(result.buckets[..31]
                .iter()
                .all(|b| b.end_exclusive_unix_ms - b.start_unix_ms == DAY));
        }
    }
}

#[test]
fn weekly_buckets_use_mondays_across_iso_year_boundary_and_clip_edges() {
    let result = statistics_buckets(LastSixMonths, date("2025-01-03"), &[], &[]).unwrap();
    assert_eq!(result.buckets[0].start_unix_ms, date("2024-07-03"));
    assert_eq!(result.buckets[0].end_exclusive_unix_ms, date("2024-07-08"));
    assert_eq!(
        result.buckets.last().unwrap().start_unix_ms,
        date("2024-12-30")
    );
    for bucket in &result.buckets[1..result.buckets.len() - 1] {
        assert_eq!((bucket.start_unix_ms / DAY + 3) % 7, 0);
        assert_eq!(bucket.end_exclusive_unix_ms - bucket.start_unix_ms, 7 * DAY);
    }
    let monday = statistics_buckets(LastYear, date("2024-03-04"), &[], &[]).unwrap();
    assert_eq!(
        monday.buckets.last().unwrap().start_unix_ms,
        date("2024-03-04")
    );
    assert_eq!(
        monday.buckets.last().unwrap().end_exclusive_unix_ms - date("2024-03-04"),
        1
    );
}

#[test]
fn sunday_and_monday_events_belong_to_different_weekly_end_values() {
    let monday = date("2024-12-30");
    let result = statistics_buckets(
        LastSixMonths,
        date("2025-01-03"),
        &[reading(monday - 1, 5), reading(monday, 11)],
        &[
            status(monday - 1, [1, 2, 3, 4, 5]),
            status(monday, [2, 3, 4, 5, 6]),
        ],
    )
    .unwrap();
    let previous = result.buckets[result.buckets.len() - 2];
    let current = result.buckets.last().unwrap();
    assert_eq!(previous.end_exclusive_unix_ms, monday);
    assert_eq!(
        previous.values.reading,
        LifetimeReadingTotals {
            japanese_occurrences: 5,
        }
    );
    assert_eq!(previous.values.statuses.known, 4);

    assert_eq!(
        current.values.reading,
        LifetimeReadingTotals {
            japanese_occurrences: 16,
        }
    );
    assert_eq!(current.values.statuses.known, 5);
}

#[test]
fn leap_days_centuries_and_calendar_year_clamping() {
    for (frame, now, expected) in [
        (LastYear, "2024-02-29", "2023-02-28"),
        (LastMonth, "2000-03-31", "2000-02-29"),
        (LastMonth, "2100-03-31", "2100-02-28"),
    ] {
        assert_eq!(
            statistics_buckets(frame, date(now), &[], &[])
                .unwrap()
                .start_unix_ms,
            date(expected)
        );
    }
    let result = statistics_buckets(
        AllTime,
        date("2025-01-03"),
        &[
            reading(date("2024-12-31"), 2),
            reading(date("2025-01-01"), 5),
        ],
        &[],
    )
    .unwrap();
    assert_eq!(result.buckets.len(), 2);
    assert_eq!(result.buckets[0].end_exclusive_unix_ms, date("2025-01-01"));
    assert_eq!(result.buckets[0].values.reading.japanese_occurrences, 2);
    assert_eq!(result.buckets[1].values.reading.japanese_occurrences, 7);
}

#[test]
fn filters_preserve_lifetime_origin_and_absolute_lines_with_exact_endpoint_ownership() {
    let start = date("2024-02-29");
    let now = date("2024-03-31");
    let readings = [
        reading(start - 1, 85_000),
        reading(start, 1_000),
        reading(now - 1, 13_000),
        reading(now, 1_000),
        reading(now + 1, 999),
    ];
    let statuses = [
        status(start - 1, [4, 5, 6, 3, 2]),
        status(start, [3, 5, 6, 4, 2]),
        status(now, [1, 2, 3, 10, 4]),
        status(now + 1, [0, 0, 0, 20, 0]),
    ];
    let result = statistics_buckets(LastMonth, now, &readings, &statuses).unwrap();
    assert_eq!(result.opening.reading.japanese_occurrences, 85_000);
    assert_eq!(result.opening.statuses.known, 3);

    assert_eq!(
        result.buckets[0].values.reading.japanese_occurrences,
        86_000
    );
    assert_eq!(result.buckets[0].values.statuses.known, 4);
    assert_eq!(result.buckets[1].values, result.buckets[0].values);
    let last = result.buckets.last().unwrap().values;
    assert_eq!(
        last.reading,
        LifetimeReadingTotals {
            japanese_occurrences: 100_000,
        }
    );
    assert_eq!(last.statuses, statuses[2].counts);

    for frame in [LastSixMonths, LastYear, AllTime] {
        assert_eq!(
            statistics_buckets(frame, now, &readings, &statuses)
                .unwrap()
                .buckets
                .last()
                .unwrap()
                .values,
            last
        );
    }
}

#[test]
fn monthly_gaps_carry_values_and_earliest_status_can_precede_reading() {
    let now = date("2024-04-01");
    let result = statistics_buckets(
        AllTime,
        now,
        &[reading(date("2024-03-01"), 3)],
        &[status(date("2024-01-01"), [4, 0, 0, 0, 0])],
    )
    .unwrap();
    assert_eq!(result.buckets.len(), 4);
    assert_eq!(result.start_unix_ms, date("2024-01-01"));
    assert_eq!(result.buckets[0].end_exclusive_unix_ms, date("2024-02-01"));
    assert_eq!(result.buckets[1].end_exclusive_unix_ms, date("2024-03-01"));
    assert_eq!(result.buckets[0].values, result.buckets[1].values);
    assert_eq!(result.buckets[1].values.reading.japanese_occurrences, 0);
    assert_eq!(result.buckets[2].values.reading.japanese_occurrences, 3);
    assert_eq!(result.buckets[2].values, result.buckets[3].values);
    assert_eq!(result.buckets[3].values.statuses.new, 4);
}

#[test]
fn epoch_empty_future_only_and_upper_calendar_limit_are_bounded() {
    for frame in [LastMonth, LastSixMonths, LastYear, AllTime] {
        let result = statistics_buckets(frame, 0, &[reading(1, 99)], &[]).unwrap();
        assert_eq!(result.start_unix_ms, 0);
        assert_eq!(result.buckets.len(), 1);
        assert_eq!(result.buckets[0].end_exclusive_unix_ms, 1);
        assert_eq!(result.buckets[0].values, Default::default());
    }
    let max = 253_402_300_799_999;
    let result = statistics_buckets(AllTime, max, &[reading(max, 1)], &[]).unwrap();
    assert_eq!(result.buckets.len(), 1);
    assert_eq!(result.buckets[0].end_exclusive_unix_ms, max + 1);
}

#[test]
fn invalid_dates_order_and_overflow_fail_without_partial_values() {
    for now in [-1, 253_402_300_800_000, i64::MAX] {
        assert!(statistics_buckets(LastYear, now, &[], &[]).is_err());
    }
    for invalid in [
        vec![reading(-1, 1)],
        vec![reading(i64::MAX, 1)],
        vec![reading(1, 1), reading(0, 1)],
        vec![reading(0, 1), reading(0, 1)],
    ] {
        assert!(statistics_buckets(LastYear, 2, &invalid, &[]).is_err());
    }
    for dates in [[-1, 1], [1, 0], [0, 0], [0, i64::MAX]] {
        let points = dates.map(|d| status(d, [0; 5]));
        assert!(statistics_buckets(LastYear, 2, &[], &points).is_err());
    }
    for increments in [[reading(0, u64::MAX), reading(1, 1)]] {
        for now in [2, date("2024-03-31")] {
            assert!(statistics_buckets(LastMonth, now, &increments, &[]).is_err());
        }
    }
}

#[test]
fn last_week_is_seven_utc_dates_with_lifetime_opening_and_inclusive_cutoff() {
    for now in [
        date("2024-03-01") + DAY / 2,
        date("2025-01-03"),
        date("2024-01-01") + DAY - 1,
    ] {
        let start = (now / DAY - 6) * DAY;
        let result = statistics_buckets(
            LastWeek,
            now,
            &[
                reading(start - 1, 4),
                reading(start, 2),
                reading(now, 3),
                reading(now + 1, 99),
            ],
            &[
                status(start - 1, [1, 2, 3, 4, 5]),
                status(now, [0, 2, 3, 5, 5]),
            ],
        )
        .unwrap();
        assert_eq!(result.start_unix_ms, start);
        assert_eq!(result.end_exclusive_unix_ms, now + 1);
        assert_eq!(result.buckets.len(), 7);
        assert_eq!(result.opening.reading.japanese_occurrences, 4);
        assert_eq!(result.opening.statuses.new, 1);
        assert_eq!(result.buckets[0].values.reading.japanese_occurrences, 6);
        assert_eq!(result.buckets[6].values.reading.japanese_occurrences, 9);
        assert_eq!(result.buckets[6].values.statuses.known, 5);
        for (index, bucket) in result.buckets.iter().enumerate() {
            assert_eq!(bucket.start_unix_ms, start + index as i64 * DAY);
            assert_eq!(
                bucket.end_exclusive_unix_ms,
                (start + (index + 1) as i64 * DAY).min(now + 1)
            );
        }
    }
    assert_eq!(
        statistics_buckets(LastWeek, 0, &[], &[])
            .unwrap()
            .buckets
            .len(),
        1
    );
}
