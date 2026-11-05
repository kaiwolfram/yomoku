//! UTC calendar projections. The caller supplies the clock; history is read only.

use super::{LifetimeReadingTotals, StatusTimelinePoint};
use crate::library::WordStatusCounts;
use serde::{Deserialize, Serialize};

const DAY_MS: i64 = 86_400_000;
// Last millisecond of Gregorian year 9999. Restrict calendar conversion and
// allocation independently of the wider durable timestamp representation.
const MAX_DATE_MS: i64 = 253_402_300_799_999;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[repr(C)]
pub enum StatisticsTimeframe {
    LastMonth,
    LastSixMonths,
    #[default]
    LastYear,
    AllTime,
}

/// Completed analyzed reading on one date, with repeated occurrences retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadingTimelinePoint {
    pub completed_at_unix_ms: i64,
    pub totals: LifetimeReadingTotals,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct StatisticsValues {
    pub reading: LifetimeReadingTotals,
    pub statuses: WordStatusCounts,
}

/// Absolute values just before `end_exclusive_unix_ms`. Empty intervals carry
/// their preceding values. The first/last calendar intervals may be clipped.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct StatisticsBucket {
    pub start_unix_ms: i64,
    pub end_exclusive_unix_ms: i64,
    pub values: StatisticsValues,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
pub struct StatisticsBuckets {
    pub timeframe: StatisticsTimeframe,
    pub start_unix_ms: i64,
    pub end_exclusive_unix_ms: i64,
    /// Lifetime state immediately before the visible range; never reset to zero
    /// by a filter. Events exactly on the range start belong to its first bucket.
    pub opening: StatisticsValues,
    pub buckets: Vec<StatisticsBucket>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidStatisticsHistory;

impl std::fmt::Display for InvalidStatisticsHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid statistics history or calendar date")
    }
}

impl std::error::Error for InvalidStatisticsHistory {}

/// Daily / Monday-start weekly / monthly end-of-bucket values. Finite ranges
/// start at UTC midnight one/six/twelve calendar months before the supplied date,
/// clamping its day to the target month. Include the supplied instant, exclude
/// later events, and fill gaps. All time starts at the earliest visible event's
/// month (the current month when empty). Input dates must be nonnegative and at
/// most year 9999; both timelines must be strictly chronological. Reading values
/// are per-date increments; status values are absolute validated replay results.
pub fn statistics_buckets(
    timeframe: StatisticsTimeframe,
    as_of_unix_ms: i64,
    reading: &[ReadingTimelinePoint],
    statuses: &[StatusTimelinePoint],
) -> Result<StatisticsBuckets, InvalidStatisticsHistory> {
    validate_dates(
        as_of_unix_ms,
        reading.iter().map(|p| p.completed_at_unix_ms),
    )?;
    validate_dates(
        as_of_unix_ms,
        statuses.iter().map(|p| p.occurred_at_unix_ms),
    )?;
    let today = Date::from_day(as_of_unix_ms / DAY_MS);
    let start = match timeframe {
        StatisticsTimeframe::LastMonth => today.months_before(1).day_number() * DAY_MS,
        StatisticsTimeframe::LastSixMonths => today.months_before(6).day_number() * DAY_MS,
        StatisticsTimeframe::LastYear => today.months_before(12).day_number() * DAY_MS,
        StatisticsTimeframe::AllTime => {
            let first = reading
                .first()
                .map(|p| p.completed_at_unix_ms)
                .into_iter()
                .chain(statuses.first().map(|p| p.occurred_at_unix_ms))
                .min()
                .unwrap_or(as_of_unix_ms)
                .min(as_of_unix_ms);
            Date {
                day: 1,
                ..Date::from_day(first / DAY_MS)
            }
            .day_number()
                * DAY_MS
        }
    }
    .max(0);
    let end = as_of_unix_ms + 1;
    let mut reading = reading.iter().peekable();
    let mut statuses = statuses.iter().peekable();
    let mut values = StatisticsValues::default();
    let mut advance = |before: i64| -> Result<StatisticsValues, InvalidStatisticsHistory> {
        while reading
            .peek()
            .is_some_and(|p| p.completed_at_unix_ms < before)
        {
            let delta = reading.next().expect("peeked reading").totals;
            values.reading.japanese_occurrences = values
                .reading
                .japanese_occurrences
                .checked_add(delta.japanese_occurrences)
                .ok_or(InvalidStatisticsHistory)?;
        }
        while statuses
            .peek()
            .is_some_and(|p| p.occurred_at_unix_ms < before)
        {
            let point = statuses.next().expect("peeked status");
            values.statuses = point.counts;
        }
        Ok(values)
    };
    let opening = advance(start)?;
    let mut buckets = Vec::new();
    let mut cursor = start;
    while cursor < end {
        let day = cursor / DAY_MS;
        let boundary = match timeframe {
            StatisticsTimeframe::LastMonth => (day + 1) * DAY_MS,
            StatisticsTimeframe::LastSixMonths | StatisticsTimeframe::LastYear => {
                // 1970-01-01 was Thursday, index 3 when Monday is zero.
                (day + 7 - (day + 3).rem_euclid(7)) * DAY_MS
            }
            StatisticsTimeframe::AllTime => Date::from_day(day).next_month().day_number() * DAY_MS,
        }
        .min(end);
        buckets.push(StatisticsBucket {
            start_unix_ms: cursor,
            end_exclusive_unix_ms: boundary,
            values: advance(boundary)?,
        });
        cursor = boundary;
    }
    Ok(StatisticsBuckets {
        timeframe,
        start_unix_ms: start,
        end_exclusive_unix_ms: end,
        opening,
        buckets,
    })
}

fn validate_dates(
    as_of: i64,
    dates: impl Iterator<Item = i64>,
) -> Result<(), InvalidStatisticsHistory> {
    if !(0..=MAX_DATE_MS).contains(&as_of) {
        return Err(InvalidStatisticsHistory);
    }
    let mut previous = -1;
    for date in dates {
        if date <= previous || date > MAX_DATE_MS {
            return Err(InvalidStatisticsHistory);
        }
        previous = date;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Date {
    year: i64,
    month: i64,
    day: i64,
}

impl Date {
    fn from_day(day: i64) -> Self {
        // Binary search avoids iterative traversal of years in long histories.
        let (mut low, mut high) = (1970, 10000);
        while low + 1 < high {
            let middle = (low + high) / 2;
            if year_start(middle) <= day {
                low = middle;
            } else {
                high = middle;
            }
        }
        let mut date = Self {
            year: low,
            month: 1,
            day: 1,
        };
        let mut remaining = day - year_start(low);
        while remaining >= date.days_in_month() {
            remaining -= date.days_in_month();
            date.month += 1;
        }
        date.day = remaining + 1;
        date
    }

    fn days_in_month(self) -> i64 {
        match self.month {
            4 | 6 | 9 | 11 => 30,
            2 if self.year % 4 == 0 && (self.year % 100 != 0 || self.year % 400 == 0) => 29,
            2 => 28,
            _ => 31,
        }
    }

    fn day_number(self) -> i64 {
        year_start(self.year)
            + (1..self.month)
                .map(|month| Self { month, ..self }.days_in_month())
                .sum::<i64>()
            + self.day
            - 1
    }

    fn months_before(self, months: i64) -> Self {
        let total = self.year * 12 + self.month - 1 - months;
        let mut date = Self {
            year: total / 12,
            month: total % 12 + 1,
            day: self.day,
        };
        date.day = date.day.min(date.days_in_month());
        date
    }

    fn next_month(self) -> Self {
        Self {
            year: self.year + i64::from(self.month == 12),
            month: self.month % 12 + 1,
            day: 1,
        }
    }
}

fn year_start(year: i64) -> i64 {
    let preceding = year - 1;
    // Gregorian days before January 1, relative to the Unix epoch.
    365 * preceding + preceding / 4 - preceding / 100 + preceding / 400 - 719_162
}
