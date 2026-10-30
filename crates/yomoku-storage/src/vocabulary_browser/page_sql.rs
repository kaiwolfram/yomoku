//! Shared production query construction; also compiled into the opt-in host profiler.

use yomoku_core::vocabulary::{VocabularySort, VOCABULARY_PAGE_SIZE};

pub(crate) fn page_sql(sort: VocabularySort, predicate: &str) -> String {
    let order = match sort {
        VocabularySort::Newest => "v.registered_at_unix_ms DESC, v.id DESC",
        VocabularySort::Frequency => {
            "encountered_count DESC, v.registered_at_unix_ms DESC, v.id DESC"
        }
        VocabularySort::LastSeen => {
            "last_seen_unix_ms DESC, v.registered_at_unix_ms DESC, v.id DESC"
        }
        VocabularySort::Alphabetical => "v.surface COLLATE BINARY ASC",
    };
    // Only the sorting metric belongs in candidate selection. Defer the other
    // summary and provenance probes until after LIMIT/OFFSET, and reuse the
    // selected metric. OFFSET and explicit inner/outer order prevent flattening.
    let metric = match sort {
        VocabularySort::Frequency => format!(", {} AS encountered_count", count("v")),
        VocabularySort::LastSeen => format!(", {} AS last_seen_unix_ms", last_seen("v")),
        _ => String::new(),
    };
    let (source, alias, suffix) = if sort != VocabularySort::Alphabetical {
        (
            format!(
                "(SELECT v.id, v.surface, v.status, v.registered_at_unix_ms{metric}
                  FROM vocabulary v WHERE {predicate}
                  ORDER BY {order} LIMIT {VOCABULARY_PAGE_SIZE} OFFSET ?) page_v"
            ),
            "page_v",
            format!("ORDER BY {}", order.replace("v.", "page_v.")),
        )
    } else {
        (
            "vocabulary v".to_owned(),
            "v",
            format!("WHERE {predicate} ORDER BY {order} LIMIT {VOCABULARY_PAGE_SIZE} OFFSET ?"),
        )
    };
    // Scalar indexed probes avoid grouping the entire occurrence table or
    // multiplying one entry by its readings/forms/events. No reservation JSON.
    let count = if sort == VocabularySort::Frequency {
        "page_v.encountered_count".to_owned()
    } else {
        count(alias)
    };
    let last_seen = if sort == VocabularySort::LastSeen {
        "page_v.last_seen_unix_ms".to_owned()
    } else {
        last_seen(alias)
    };
    format!(
        "SELECT {alias}.id, {alias}.surface, {alias}.status, {alias}.registered_at_unix_ms,
            {count} AS encountered_count,
            {last_seen} AS last_seen_unix_ms,
            NOT EXISTS (SELECT 1 FROM reading_occurrences o
             LEFT JOIN reading_completions c USING (command_id)
             LEFT JOIN completion_analyses a USING (command_id)
             LEFT JOIN effective_completion_commands e USING (command_id)
             WHERE o.vocabulary_id = {alias}.id AND
                (c.semantics_version IS NOT 1 OR a.semantics_version IS NOT 1
                 OR (e.semantics_version IS NOT NULL AND e.semantics_version <> 1)
                 OR c.completed_at_unix_ms < 0)) AS valid_history
         FROM {source} {suffix}"
    )
}

fn count(alias: &str) -> String {
    format!("(SELECT COUNT(*) FROM reading_occurrences o WHERE o.vocabulary_id = {alias}.id)")
}

fn last_seen(alias: &str) -> String {
    format!(
        "(SELECT MAX(c.completed_at_unix_ms) FROM reading_occurrences o
         JOIN reading_completions c USING (command_id) WHERE o.vocabulary_id = {alias}.id)"
    )
}
