use yomoku_core::statistics::LifetimeReadingTotals;

use crate::{Storage, StorageError};

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
               OR (effective_version IS NOT NULL AND effective_version <> 1)
               OR active_time_ms < 0
        )";

impl Storage {
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
