use yomoku_core::{
    document::VocabularyId,
    statistics::{
        LifetimeReadingTotals, StatusTimelineBuilder, StatusTimelinePoint, VocabularyStatusChange,
    },
    vocabulary::VocabularyStatus,
};

use crate::{vocabulary::decode_status, Storage, StorageError};

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
    /// Replay surviving registration/manual/completion history in a single read
    /// snapshot. Counts are absolute, coalesced per timestamp, without date bins.
    /// Document deletion preserves points; vocabulary deletion recalculates them.
    /// Durable dictionary-form links share the same snapshot. No content,
    /// analyzer, dictionary data, cached counts or JSON is loaded.
    pub fn vocabulary_status_timeline(&self) -> Result<Vec<StatusTimelinePoint>, StorageError> {
        // Read each link once before replay, without multiplying events by their
        // surface's link count. UNION keeps both inputs in one read snapshot.
        let mut statement = self.connection.prepare(
            "SELECT e.id, e.vocabulary_id, e.semantics_version, e.old_status,
                    e.new_status, e.cause, e.occurred_at_unix_ms, e.completion_command_id,
                    c.semantics_version, a.semantics_version, r.semantics_version,
                    NULL AS dictionary_form, 1 AS row_kind
             FROM vocabulary_status_events e
             LEFT JOIN reading_completions c ON c.command_id = e.completion_command_id
             LEFT JOIN completion_analyses a ON a.command_id = e.completion_command_id
             LEFT JOIN effective_completion_commands r ON r.command_id = e.completion_command_id
             UNION ALL
             SELECT NULL, vocabulary_id, NULL, NULL, NULL, NULL, NULL, NULL,
                    NULL, NULL, NULL, dictionary_form, 0
             FROM vocabulary_dictionary_forms
             ORDER BY row_kind, id",
        )?;
        let mut rows = statement.query([])?;
        let mut timeline = StatusTimelineBuilder::default();
        while let Some(row) = rows.next()? {
            if row.get::<_, i64>(12)? == 0 {
                timeline
                    .link_dictionary_form(
                        VocabularyId::try_from(row.get::<_, i64>(1)?)
                            .map_err(|_| StorageError::InvalidVocabulary)?,
                        row.get(11)?,
                    )
                    .map_err(|_| StorageError::InvalidVocabulary)?;
                continue;
            }
            let version: i64 = row.get(2)?;
            let old: Option<String> = row.get(3)?;
            let old = old.as_deref().map(decode_status).transpose()?;
            let new = decode_status(&row.get::<_, String>(4)?)?;
            let cause: String = row.get(5)?;
            let command: Option<i64> = row.get(7)?;
            let completion_version: Option<i64> = row.get(8)?;
            let analysis_version: Option<i64> = row.get(9)?;
            let effective_version: Option<i64> = row.get(10)?;
            let valid_cause = match cause.as_str() {
                "registration" => {
                    old.is_none() && new == VocabularyStatus::New && command.is_none()
                }
                "manual" => old.is_some() && old != Some(new) && command.is_none(),
                "completion" => {
                    old == Some(VocabularyStatus::New)
                        && new == VocabularyStatus::Known
                        && command.is_some()
                        && completion_version == Some(1)
                        && analysis_version == Some(1)
                        && effective_version.is_none_or(|version| version == 1)
                }
                _ => false,
            };
            if version != 1 || !valid_cause {
                return Err(StorageError::InvalidVocabulary);
            }
            timeline
                .push(VocabularyStatusChange {
                    event_id: u64::try_from(row.get::<_, i64>(0)?)
                        .map_err(|_| StorageError::InvalidVocabulary)?,
                    vocabulary_id: VocabularyId::try_from(row.get::<_, i64>(1)?)
                        .map_err(|_| StorageError::InvalidVocabulary)?,
                    occurred_at_unix_ms: row.get(6)?,
                    old_status: old,
                    new_status: new,
                })
                .map_err(|_| StorageError::InvalidVocabulary)?;
        }
        timeline
            .finish()
            .map_err(|_| StorageError::InvalidVocabulary)
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
