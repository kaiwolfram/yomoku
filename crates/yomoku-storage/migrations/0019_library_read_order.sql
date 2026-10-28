-- Ordinal recency avoids wall-clock changes and retains one row per live document.
-- Legacy read history cannot be inferred from positions; leave it unknown.
ALTER TABLE documents ADD COLUMN last_read_sequence INTEGER
    CHECK (last_read_sequence IS NULL OR
           (typeof(last_read_sequence) = 'integer' AND last_read_sequence > 0));
CREATE TABLE library_read_clock (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    sequence INTEGER NOT NULL CHECK (typeof(sequence) = 'integer' AND sequence >= 0)
);
INSERT INTO library_read_clock (id, sequence) VALUES (1, 0);
