-- Keep historical provenance and allocation high-water marks intact.
CREATE TEMP TABLE epub_source_sequence AS SELECT seq FROM sqlite_sequence WHERE name = 'sources';
CREATE TABLE new_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    kind TEXT NOT NULL CHECK (kind IN ('paste', 'txt', 'srt', 'epub')),
    original_title TEXT NOT NULL
);
INSERT INTO new_sources SELECT id, kind, original_title FROM sources;
DROP TABLE sources;
ALTER TABLE new_sources RENAME TO sources;
UPDATE sqlite_sequence SET seq = MAX(seq, COALESCE((SELECT seq FROM epub_source_sequence), 0))
    WHERE name = 'sources';
DELETE FROM sqlite_sequence WHERE name = 'sources' AND NOT EXISTS (SELECT 1 FROM epub_source_sequence);
DROP TABLE epub_source_sequence;

-- EPUB is a container; its individual XML encodings are not a file text encoding.
CREATE TABLE new_imported_files (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    sha256 BLOB NOT NULL UNIQUE CHECK (length(sha256) = 32),
    original_bytes BLOB NOT NULL,
    text_encoding TEXT NOT NULL CHECK (text_encoding IN ('utf8', 'shift_jis', 'epub'))
) STRICT;
INSERT INTO new_imported_files SELECT * FROM imported_files;
DROP TABLE imported_files;
ALTER TABLE new_imported_files RENAME TO imported_files;

CREATE TABLE epub_documents (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    metadata_title TEXT,
    cover_path TEXT,
    chapter_count INTEGER NOT NULL CHECK (chapter_count > 0),
    asset_count INTEGER NOT NULL CHECK (asset_count >= 0)
) STRICT;

CREATE TABLE epub_assets (
    document_id INTEGER NOT NULL REFERENCES epub_documents(document_id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    media_type TEXT NOT NULL,
    bytes BLOB NOT NULL,
    sha256 BLOB NOT NULL CHECK (length(sha256) = 32),
    PRIMARY KEY (document_id, path)
) STRICT;

CREATE TABLE epub_chapters (
    chapter_id INTEGER PRIMARY KEY REFERENCES chapter_texts(chapter_id) ON DELETE CASCADE,
    document_id INTEGER NOT NULL REFERENCES epub_documents(document_id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    text_sha256 BLOB NOT NULL CHECK (length(text_sha256) = 32),
    ruby_count INTEGER NOT NULL CHECK (ruby_count >= 0),
    span_count INTEGER NOT NULL CHECK (span_count >= 0),
    image_count INTEGER NOT NULL CHECK (image_count >= 0),
    UNIQUE (document_id, path),
    UNIQUE (chapter_id, document_id)
) STRICT;

CREATE TABLE epub_spans (
    chapter_id INTEGER NOT NULL REFERENCES epub_chapters(chapter_id) ON DELETE CASCADE,
    span_index INTEGER NOT NULL CHECK (span_index >= 0),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    kind TEXT NOT NULL CHECK (kind IN ('heading', 'emphasis', 'strong', 'external', 'inert')),
    heading_level INTEGER,
    target TEXT,
    CHECK ((kind = 'heading' AND heading_level IS NOT NULL AND heading_level BETWEEN 1 AND 6 AND target IS NULL)
        OR (kind IN ('emphasis', 'strong') AND heading_level IS NULL AND target IS NULL)
        OR (kind IN ('external', 'inert') AND heading_level IS NULL AND target IS NOT NULL)),
    PRIMARY KEY (chapter_id, span_index)
) STRICT;

CREATE TABLE epub_inline_images (
    chapter_id INTEGER NOT NULL,
    document_id INTEGER NOT NULL,
    image_index INTEGER NOT NULL CHECK (image_index >= 0),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    path TEXT NOT NULL,
    alt TEXT NOT NULL,
    FOREIGN KEY (chapter_id, document_id) REFERENCES epub_chapters(chapter_id, document_id) ON DELETE CASCADE,
    FOREIGN KEY (document_id, path) REFERENCES epub_assets(document_id, path),
    PRIMARY KEY (chapter_id, image_index)
) STRICT;
