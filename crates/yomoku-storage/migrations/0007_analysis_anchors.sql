-- Immutable base analyses: manual boundaries never replace analyzer output.
CREATE TABLE chapter_analyses (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    chapter_id INTEGER NOT NULL REFERENCES chapter_texts(chapter_id) ON DELETE CASCADE,
    engine_revision TEXT NOT NULL CHECK (length(CAST(engine_revision AS BLOB)) > 0),
    dictionary_revision TEXT NOT NULL CHECK (length(CAST(dictionary_revision AS BLOB)) > 0),
    split_mode TEXT NOT NULL CHECK (split_mode IN ('a', 'b', 'c')),
    UNIQUE (chapter_id, engine_revision, dictionary_revision, split_mode)
) STRICT;

CREATE TABLE analyzed_tokens (
    analysis_id INTEGER NOT NULL REFERENCES chapter_analyses(id) ON DELETE CASCADE,
    token_index INTEGER NOT NULL CHECK (token_index >= 0),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    surface TEXT NOT NULL CHECK (length(CAST(surface AS BLOB)) > 0),
    is_japanese INTEGER NOT NULL CHECK (is_japanese IN (0, 1)),
    reading TEXT CHECK (reading IS NULL OR length(CAST(reading AS BLOB)) > 0),
    dictionary_form TEXT CHECK (dictionary_form IS NULL OR length(CAST(dictionary_form AS BLOB)) > 0),
    PRIMARY KEY (analysis_id, token_index)
) STRICT;
