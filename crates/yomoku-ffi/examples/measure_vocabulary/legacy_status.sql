-- Frozen pre-optimization production query for the opt-in comparison only.
SELECT e.id, e.vocabulary_id, e.semantics_version, e.old_status,
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
ORDER BY row_kind, id
