-- Reservation freezes complete effective boundaries/metadata and start-owned
-- occurrences. No live content/override/vocabulary FK: retries and provenance
-- survive document deletion and later override edits. Unfinished reservations
-- are never replayed automatically. Version 1 describes this JSON schema.
CREATE TABLE effective_completion_commands (
    command_id INTEGER PRIMARY KEY REFERENCES completion_commands(id),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    payload TEXT NOT NULL CHECK (json_valid(payload))
) STRICT;
