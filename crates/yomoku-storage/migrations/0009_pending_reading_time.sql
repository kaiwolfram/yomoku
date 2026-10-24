-- Only durations survive process/reboot, never monotonic clock timestamps.
-- Offsets stay stable across reflow. Zero rows retain revisions after completion
-- so a late checkpoint cannot resurrect time that has already been consumed.
CREATE TABLE pending_reading_time (
    document_id INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chapter_id INTEGER NOT NULL REFERENCES chapters(id),
    byte_offset INTEGER NOT NULL CHECK (byte_offset >= 0),
    milliseconds INTEGER NOT NULL CHECK (milliseconds >= 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    PRIMARY KEY (document_id, chapter_id, byte_offset)
) STRICT;

-- Older repository-only events had no timer; preserve them with zero duration.
ALTER TABLE reading_completions ADD COLUMN active_time_ms INTEGER NOT NULL
    DEFAULT 0 CHECK (active_time_ms >= 0);
