-- Record before external submission. No document/vocabulary foreign keys:
-- deleting reader data must never erase an unresolved external write.
CREATE TABLE anki_submissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    payload TEXT NOT NULL CHECK (length(CAST(payload AS BLOB)) > 0),
    state TEXT NOT NULL CHECK (state IN ('pending', 'uncertain', 'not_inserted', 'added', 'existing')),
    known_note_id INTEGER CHECK (known_note_id > 0),
    existing_ids TEXT,
    CHECK (state != 'pending' OR known_note_id IS NULL),
    CHECK (state != 'not_inserted' OR known_note_id IS NULL),
    CHECK (state != 'added' OR known_note_id IS NOT NULL),
    CHECK ((state = 'existing' AND existing_ids IS NOT NULL) OR
           (state != 'existing' AND existing_ids IS NULL))
) STRICT;

-- The retained export owner permits only one unresolved submission app-wide.
CREATE UNIQUE INDEX anki_one_unresolved ON anki_submissions ((1))
    WHERE state IN ('pending', 'uncertain');
