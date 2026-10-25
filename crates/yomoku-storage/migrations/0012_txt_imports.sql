-- Widen the original paste-only CHECK using SQLite's table-rebuild procedure.
-- The migration runner disables FKs outside BEGIN and checks them before COMMIT.
CREATE TEMP TABLE txt_source_sequence AS SELECT seq FROM sqlite_sequence WHERE name = 'sources';
CREATE TABLE new_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    kind TEXT NOT NULL CHECK (kind IN ('paste', 'txt')),
    original_title TEXT NOT NULL
);
INSERT INTO new_sources SELECT id, kind, original_title FROM sources;
DROP TABLE sources;
ALTER TABLE new_sources RENAME TO sources;
UPDATE sqlite_sequence SET seq = MAX(seq, COALESCE((SELECT seq FROM txt_source_sequence), 0))
    WHERE name = 'sources';
DELETE FROM sqlite_sequence WHERE name = 'sources' AND NOT EXISTS (SELECT 1 FROM txt_source_sequence);
DROP TABLE txt_source_sequence;

-- Identity covers original bytes, including encoding/BOM/markup/line endings.
-- Live file identity is separate from pasted canonical UTF-8 identity.
CREATE TABLE imported_files (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    sha256 BLOB NOT NULL UNIQUE CHECK (length(sha256) = 32),
    original_bytes BLOB NOT NULL,
    text_encoding TEXT NOT NULL CHECK (text_encoding IN ('utf8', 'shift_jis'))
) STRICT;

CREATE TABLE source_ruby (
    chapter_id INTEGER NOT NULL REFERENCES chapter_texts(chapter_id) ON DELETE CASCADE,
    ruby_index INTEGER NOT NULL CHECK (ruby_index >= 0),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    reading TEXT NOT NULL CHECK (length(CAST(reading AS BLOB)) > 0),
    PRIMARY KEY (chapter_id, ruby_index)
) STRICT;
