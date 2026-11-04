use yomoku_core::statistics::{
    statistics_buckets, LifetimeReadingTotals, ReadingTimelinePoint, StatisticsBuckets,
    StatisticsTimeframe, StatusTimelineBuilder, StatusTimelinePoint,
};

use crate::{vocabulary::decode_status, Storage, StorageError};

mod status_row;

pub(super) const STATUS_EVENTS_SQL: &str = include_str!("statistics/status_events.sql");

// One statement gives both totals the same SQLite snapshot. Sum time separately
// from occurrences: joining before SUM would multiply repeated words' time and
// drop zero-word pages. No live document, analysis, vocabulary or JSON is loaded.
const LIFETIME_SQL: &str = "
    WITH completed AS (
        SELECT c.command_id, c.active_time_ms,
               c.semantics_version AS completion_version,
               a.semantics_version AS analysis_version,
               e.semantics_version AS effective_version
        FROM reading_completions c
        JOIN completion_analyses a USING (command_id)
        LEFT JOIN effective_completion_commands e USING (command_id)
    )
    SELECT
        (SELECT COUNT(*) FROM reading_occurrences JOIN completed USING (command_id)),
        (SELECT COALESCE(SUM(active_time_ms), 0) FROM completed),
        NOT EXISTS (
            SELECT 1 FROM completed
            WHERE completion_version <> 1 OR analysis_version <> 1
               OR (effective_version IS NOT NULL AND effective_version <> 2)
               OR active_time_ms < 0
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
    /// Unknown event versions/corrupt durations fail; SQLite integer SUM overflow
    /// returns an error instead of approximating or truncating milliseconds.
    pub fn lifetime_reading_totals(&self) -> Result<LifetimeReadingTotals, StorageError> {
        let (occurrences, milliseconds, valid) =
            self.connection.query_row(LIFETIME_SQL, [], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })?;
        if !valid {
            return Err(StorageError::InvalidCompletion);
        }
        Ok(LifetimeReadingTotals {
            japanese_occurrences: u64::try_from(occurrences)
                .map_err(|_| StorageError::InvalidCompletion)?,
            active_time_ms: u64::try_from(milliseconds)
                .map_err(|_| StorageError::InvalidCompletion)?,
        })
    }
}

fn reading_timeline(
    connection: &rusqlite::Connection,
) -> Result<Vec<ReadingTimelinePoint>, StorageError> {
    // Aggregate time once per command, including zero-occurrence pages. Count
    // only surviving actual occurrences, never the frozen reservation payload.
    let mut statement = connection.prepare(
        "SELECT c.completed_at_unix_ms,
                SUM((SELECT COUNT(*) FROM reading_occurrences o
                     WHERE o.command_id = c.command_id)),
                SUM(c.active_time_ms),
                MIN(c.semantics_version = 1 AND a.semantics_version = 1
                    AND (e.semantics_version IS NULL OR e.semantics_version = 2)
                    AND c.active_time_ms >= 0)
         FROM reading_completions c
         JOIN completion_analyses a USING (command_id)
         LEFT JOIN effective_completion_commands e USING (command_id)
         GROUP BY c.completed_at_unix_ms
         ORDER BY c.completed_at_unix_ms",
    )?;
    let mut rows = statement.query([])?;
    let mut points = Vec::new();
    let mut lifetime = [0_i64; 2];
    while let Some(row) = rows.next()? {
        if !row.get::<_, bool>(3)? {
            return Err(StorageError::InvalidCompletion);
        }
        let increments = [row.get::<_, i64>(1)?, row.get::<_, i64>(2)?];
        // Match the existing lifetime SQL's exact signed-integer limit even
        // when individually valid date groups would overflow its total SUM.
        for (total, increment) in lifetime.iter_mut().zip(increments) {
            if increment < 0 {
                return Err(StorageError::InvalidCompletion);
            }
            *total = total
                .checked_add(increment)
                .ok_or(StorageError::InvalidCompletion)?;
        }
        points.push(ReadingTimelinePoint {
            completed_at_unix_ms: row.get(0)?,
            totals: LifetimeReadingTotals {
                japanese_occurrences: u64::try_from(increments[0])
                    .map_err(|_| StorageError::InvalidCompletion)?,
                active_time_ms: u64::try_from(increments[1])
                    .map_err(|_| StorageError::InvalidCompletion)?,
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
