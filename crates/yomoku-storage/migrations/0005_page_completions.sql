-- IDs are issued before submission, so retries (including after reopen) reuse
-- one identity. Unsubmitted IDs are not completed-reading history.
CREATE TABLE completion_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0)
) STRICT;

-- Document ID is a historical value, deliberately without a live-document FK.
-- Source/chapter provenance survives future document/content deletion.
CREATE TABLE reading_completions (
    command_id INTEGER PRIMARY KEY REFERENCES completion_commands(id),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    document_id INTEGER NOT NULL CHECK (document_id > 0),
    source_id INTEGER NOT NULL REFERENCES sources(id),
    chapter_id INTEGER NOT NULL REFERENCES chapters(id),
    start_byte_offset INTEGER NOT NULL CHECK (start_byte_offset >= 0),
    end_byte_offset INTEGER NOT NULL CHECK (end_byte_offset >= start_byte_offset),
    departing_byte_offset INTEGER NOT NULL
        CHECK (departing_byte_offset >= start_byte_offset AND departing_byte_offset <= end_byte_offset),
    page_length INTEGER NOT NULL CHECK (page_length IN (250, 500, 750, 1000, 1500)),
    completed_at_unix_ms INTEGER NOT NULL CHECK (completed_at_unix_ms >= 0)
) STRICT;

CREATE INDEX reading_completions_by_date ON reading_completions(completed_at_unix_ms, command_id);
