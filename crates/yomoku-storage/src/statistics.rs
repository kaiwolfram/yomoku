use yomoku_core::statistics::{
    statistics_buckets, LifetimeReadingTotals, ReadingTimelinePoint, StatisticsBuckets,
    StatisticsTimeframe, StatusTimelineBuilder, StatusTimelinePoint,
};

use crate::{vocabulary::decode_status, Storage, StorageError};

mod status_row;

pub(super) const STATUS_EVENTS_SQL: &str = include_str!("statistics/status_events.sql");

// Count surviving occurrences in committed analyzed history without loading live content.
const LIFETIME_SQL: &str = "
    WITH completed AS (
        SELECT c.command_id,
               c.semantics_version AS completion_version,
               a.semantics_version AS analysis_version
        FROM reading_completions c
        JOIN completion_analyses a USING (command_id)
    )
    SELECT
        (SELECT COUNT(*) FROM reading_occurrences JOIN completed USING (command_id)),
        NOT EXISTS (
            SELECT 1 FROM completed
            WHERE completion_version <> 1 OR analysis_version <> 1
        )";

impl Storage {
    /// Calendar projections from one deferred read transaction: all chart lines
    /// see the same committed history. The caller supplies the UTC clock. No
    /// analyzer, live content, JSON, caches, registrations or database writes.
    pub fn statistics_buckets(
        &mut self,
        timeframe: StatisticsTimeframe,
        as_of_unix_ms: i64,
    ) -> Result<StatisticsBuckets, StorageError> {
        let transaction = self.connection.transaction()?;
        let statuses = status_timeline(&transaction)?;
        let reading = reading_timeline(&transaction)?;
        let buckets = statistics_buckets(timeframe, as_of_unix_ms, &reading, &statuses)
            .map_err(|_| StorageError::InvalidStatistics)?;
        transaction.commit()?;
        Ok(buckets)
    }

    /// Replay surviving registration/manual/completion history in a single read
    /// snapshot. Counts are absolute, coalesced per timestamp, without date bins.
    /// Document deletion preserves points; vocabulary deletion recalculates them.
    /// No content,
    /// analyzer, dictionary data, cached counts or JSON is loaded.
    pub fn vocabulary_status_timeline(&self) -> Result<Vec<StatusTimelinePoint>, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let points = status_timeline(&transaction)?;
        transaction.commit()?;
        Ok(points)
    }

    /// Derive exact lifetime totals from committed analyzed history, including
    /// deleted documents. No registration, cached counters, analyzer or writes.
    /// Legacy events without analysis markers have no inferred statistics.
    /// Unknown event versions fail rather than silently returning partial history.
    pub fn lifetime_reading_totals(&self) -> Result<LifetimeReadingTotals, StorageError> {
        let (occurrences, valid) = self.connection.query_row(LIFETIME_SQL, [], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?))
        })?;
        if !valid {
            return Err(StorageError::InvalidCompletion);
        }
        Ok(LifetimeReadingTotals {
            japanese_occurrences: u64::try_from(occurrences)
                .map_err(|_| StorageError::InvalidCompletion)?,
        })
    }
}

fn reading_timeline(
    connection: &rusqlite::Connection,
) -> Result<Vec<ReadingTimelinePoint>, StorageError> {
    // Count surviving occurrences from committed history.
    let mut statement = connection.prepare(
        "SELECT c.completed_at_unix_ms,
                SUM((SELECT COUNT(*) FROM reading_occurrences o
                     WHERE o.command_id = c.command_id)),
                MIN(c.semantics_version = 1 AND a.semantics_version = 1)
         FROM reading_completions c
         JOIN completion_analyses a USING (command_id)
         GROUP BY c.completed_at_unix_ms
         ORDER BY c.completed_at_unix_ms",
    )?;
    let mut rows = statement.query([])?;
    let mut points = Vec::new();
    let mut lifetime = 0_i64;
    while let Some(row) = rows.next()? {
        if !row.get::<_, bool>(2)? {
            return Err(StorageError::InvalidCompletion);
        }
        let occurrences = row.get::<_, i64>(1)?;
        if occurrences < 0 {
            return Err(StorageError::InvalidCompletion);
        }
        lifetime = lifetime
            .checked_add(occurrences)
            .ok_or(StorageError::InvalidCompletion)?;
        points.push(ReadingTimelinePoint {
            completed_at_unix_ms: row.get(0)?,
            totals: LifetimeReadingTotals {
                japanese_occurrences: occurrences as u64,
            },
        });
    }
    Ok(points)
}

pub(super) fn status_timeline(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<Vec<StatusTimelinePoint>, StorageError> {
    let mut timeline = StatusTimelineBuilder::default();
    let mut statement = transaction.prepare(STATUS_EVENTS_SQL)?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        timeline
            .push(status_row::decode(row)?)
            .map_err(|_| StorageError::InvalidVocabulary)?;
    }
    timeline
        .finish()
        .map_err(|_| StorageError::InvalidVocabulary)
}
