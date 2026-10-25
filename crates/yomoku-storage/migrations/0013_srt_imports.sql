-- Preserve provenance IDs and the AUTOINCREMENT high-water mark while widening kinds.
CREATE TEMP TABLE srt_source_sequence AS SELECT seq FROM sqlite_sequence WHERE name = 'sources';
CREATE TABLE new_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    kind TEXT NOT NULL CHECK (kind IN ('paste', 'txt', 'srt')),
    original_title TEXT NOT NULL
);
INSERT INTO new_sources SELECT id, kind, original_title FROM sources;
DROP TABLE sources;
ALTER TABLE new_sources RENAME TO sources;
UPDATE sqlite_sequence SET seq = MAX(seq, COALESCE((SELECT seq FROM srt_source_sequence), 0))
    WHERE name = 'sources';
DELETE FROM sqlite_sequence WHERE name = 'sources' AND NOT EXISTS (SELECT 1 FROM srt_source_sequence);
DROP TABLE srt_source_sequence;

-- Canonical UTF-8 cue ranges exclude the blank separator between cues.
CREATE TABLE subtitle_cues (
    chapter_id INTEGER NOT NULL REFERENCES chapter_texts(chapter_id) ON DELETE CASCADE,
    cue_index INTEGER NOT NULL CHECK (cue_index >= 0),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    PRIMARY KEY (chapter_id, cue_index)
) STRICT;
