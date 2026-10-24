-- Presence distinguishes complete analyzed history from legacy time/position
-- events. Analysis identity is historical: deleting live text must retain it.
CREATE TABLE completion_analyses (
    command_id INTEGER PRIMARY KEY REFERENCES reading_completions(command_id),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    analysis_id INTEGER NOT NULL CHECK (analysis_id > 0),
    engine_revision TEXT NOT NULL CHECK (length(CAST(engine_revision AS BLOB)) > 0),
    dictionary_revision TEXT NOT NULL CHECK (length(CAST(dictionary_revision AS BLOB)) > 0),
    split_mode TEXT NOT NULL CHECK (split_mode IN ('a', 'b', 'c'))
) STRICT;

-- Preserve each repetition and its full immutable analyzer anchor/lemma.
-- Vocabulary deletion erases its encounters; document deletion preserves them.
CREATE TABLE reading_occurrences (
    command_id INTEGER NOT NULL REFERENCES completion_analyses(command_id),
    token_index INTEGER NOT NULL CHECK (token_index >= 0),
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    surface TEXT NOT NULL CHECK (length(CAST(surface AS BLOB)) > 0),
    reading TEXT CHECK (reading IS NULL OR length(CAST(reading AS BLOB)) > 0),
    dictionary_form TEXT CHECK (dictionary_form IS NULL OR length(CAST(dictionary_form AS BLOB)) > 0),
    PRIMARY KEY (command_id, token_index)
) STRICT;
CREATE INDEX reading_occurrences_by_entry ON reading_occurrences(vocabulary_id, command_id);

-- Extend the existing dated timeline, retaining event IDs and the allocation
-- high-water mark even when its highest event was previously deleted.
CREATE TABLE vocabulary_status_events_next (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    old_status TEXT CHECK (old_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    new_status TEXT NOT NULL CHECK (new_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    cause TEXT NOT NULL CHECK (cause IN ('registration', 'manual', 'completion')),
    occurred_at_unix_ms INTEGER NOT NULL CHECK (occurred_at_unix_ms >= 0),
    completion_command_id INTEGER REFERENCES completion_analyses(command_id),
    CHECK ((cause = 'registration' AND old_status IS NULL AND new_status = 'new' AND completion_command_id IS NULL)
        OR (cause = 'manual' AND old_status IS NOT NULL AND old_status <> new_status AND completion_command_id IS NULL)
        OR (cause = 'completion' AND old_status IS NOT NULL AND old_status = 'new' AND new_status = 'known' AND completion_command_id IS NOT NULL))
) STRICT;
INSERT INTO vocabulary_status_events_next
    (id, semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms)
    SELECT id, semantics_version, vocabulary_id, old_status, new_status, cause, occurred_at_unix_ms
    FROM vocabulary_status_events;
UPDATE sqlite_sequence SET seq = MAX(seq, COALESCE(
    (SELECT seq FROM sqlite_sequence WHERE name = 'vocabulary_status_events'), 0))
    WHERE name = 'vocabulary_status_events_next';
DROP TABLE vocabulary_status_events;
ALTER TABLE vocabulary_status_events_next RENAME TO vocabulary_status_events;
CREATE INDEX vocabulary_status_events_by_date ON vocabulary_status_events(occurred_at_unix_ms, id);
CREATE INDEX vocabulary_status_events_by_entry ON vocabulary_status_events(vocabulary_id, id);
CREATE UNIQUE INDEX vocabulary_completion_status_once
    ON vocabulary_status_events(completion_command_id, vocabulary_id) WHERE cause = 'completion';
