-- Exact global spelling; independent of registration/status and source lifetime.
-- No built-in payload, analyzer reading or historical occurrence is edited here.
CREATE TABLE custom_dictionary_entries (
    surface TEXT NOT NULL COLLATE BINARY PRIMARY KEY CHECK (length(CAST(surface AS BLOB)) > 0),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    english_meaning TEXT NOT NULL CHECK (length(CAST(english_meaning AS BLOB)) > 0),
    reading TEXT NOT NULL CHECK (length(CAST(reading AS BLOB)) > 0)
) STRICT, WITHOUT ROWID;
