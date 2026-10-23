-- Global identity is exact written spelling, independent of document/reading.
CREATE TABLE vocabulary (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    surface TEXT NOT NULL COLLATE BINARY UNIQUE CHECK (length(CAST(surface AS BLOB)) > 0),
    status TEXT NOT NULL CHECK (status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    registered_at_unix_ms INTEGER NOT NULL CHECK (registered_at_unix_ms >= 0)
) STRICT;

-- Dated registration and actual status changes, never lookup/reading counts.
-- Cascading vocabulary-history deletion is distinct from document deletion.
CREATE TABLE vocabulary_status_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    old_status TEXT CHECK (old_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    new_status TEXT NOT NULL CHECK (new_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    cause TEXT NOT NULL CHECK (cause IN ('registration', 'manual')),
    occurred_at_unix_ms INTEGER NOT NULL CHECK (occurred_at_unix_ms >= 0),
    CHECK ((cause = 'registration' AND old_status IS NULL AND new_status = 'new')
        OR (cause = 'manual' AND old_status IS NOT NULL AND old_status <> new_status))
) STRICT;

CREATE INDEX vocabulary_status_events_by_date
    ON vocabulary_status_events(occurred_at_unix_ms, id);
CREATE INDEX vocabulary_status_events_by_entry
    ON vocabulary_status_events(vocabulary_id, id);
