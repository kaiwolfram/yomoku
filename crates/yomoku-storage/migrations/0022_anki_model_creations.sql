-- Model creation can leave a partial external model. Retain the exact intent
-- independently of reader/vocabulary lifetimes; this is not a replay queue.
CREATE TABLE anki_model_creations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    payload TEXT NOT NULL CHECK (length(CAST(payload AS BLOB)) > 0),
    state TEXT NOT NULL CHECK (state IN ('pending', 'uncertain', 'not_created', 'created', 'existing')),
    known_model_id INTEGER CHECK (known_model_id > 0),
    existing_model_id INTEGER CHECK (existing_model_id > 0),
    CHECK (state NOT IN ('pending', 'not_created') OR known_model_id IS NULL),
    CHECK (state != 'created' OR known_model_id IS NOT NULL),
    CHECK ((state = 'existing' AND existing_model_id IS NOT NULL) OR
           (state != 'existing' AND existing_model_id IS NULL))
) STRICT;

CREATE UNIQUE INDEX anki_one_unresolved_model ON anki_model_creations ((1))
    WHERE state IN ('pending', 'uncertain');
