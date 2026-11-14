-- Initial release schema. Development databases and backups are unsupported.

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY CHECK (version > 0),
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    source_id INTEGER NOT NULL REFERENCES sources(id),
    chapter_index INTEGER NOT NULL CHECK (chapter_index >= 0),
    UNIQUE (source_id, chapter_index)
);

CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    source_id INTEGER NOT NULL UNIQUE REFERENCES sources(id),
    title TEXT NOT NULL
, finished_at_least_once INTEGER NOT NULL DEFAULT 0
    CHECK (finished_at_least_once IN (0, 1)), last_read_sequence INTEGER
    CHECK (last_read_sequence IS NULL OR
           (typeof(last_read_sequence) = 'integer' AND last_read_sequence > 0)));

CREATE TABLE chapter_texts (
    chapter_id INTEGER PRIMARY KEY REFERENCES chapters(id),
    text TEXT NOT NULL
);

CREATE TABLE reading_positions (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    chapter_id INTEGER NOT NULL REFERENCES chapters(id),
    byte_offset INTEGER NOT NULL CHECK (byte_offset >= 0)
);

CREATE TABLE reader_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    page_length INTEGER NOT NULL CHECK (page_length IN (250, 500, 750, 1000, 1500))
, font_size INTEGER NOT NULL DEFAULT 16
    CHECK (font_size IN (14, 16, 18, 20, 24)), line_spacing INTEGER NOT NULL DEFAULT 150
    CHECK (line_spacing IN (125, 150, 175, 200))) STRICT;

CREATE TABLE completion_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0)
) STRICT;

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
    completed_at_unix_ms INTEGER NOT NULL CHECK (completed_at_unix_ms >= 0)) STRICT;

CREATE TABLE vocabulary (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    surface TEXT NOT NULL COLLATE BINARY UNIQUE CHECK (length(CAST(surface AS BLOB)) > 0),
    status TEXT NOT NULL CHECK (status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    registered_at_unix_ms INTEGER NOT NULL CHECK (registered_at_unix_ms >= 0)
) STRICT;

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

CREATE TABLE paste_content_hashes (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    sha256 BLOB NOT NULL CHECK (length(sha256) = 32)
) STRICT;

CREATE TABLE completion_analyses (
    command_id INTEGER PRIMARY KEY REFERENCES reading_completions(command_id),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    analysis_id INTEGER NOT NULL CHECK (analysis_id > 0),
    engine_revision TEXT NOT NULL CHECK (length(CAST(engine_revision AS BLOB)) > 0),
    dictionary_revision TEXT NOT NULL CHECK (length(CAST(dictionary_revision AS BLOB)) > 0),
    split_mode TEXT NOT NULL CHECK (split_mode IN ('a', 'b', 'c'))
) STRICT;

CREATE TABLE reading_occurrences (
    command_id INTEGER NOT NULL REFERENCES completion_analyses(command_id),
    token_index INTEGER NOT NULL CHECK (token_index >= 0),
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte),
    surface TEXT NOT NULL CHECK (length(CAST(surface AS BLOB)) > 0),
    reading TEXT CHECK (reading IS NULL OR length(CAST(reading AS BLOB)) > 0),
    dictionary_form TEXT CHECK (dictionary_form IS NULL OR length(CAST(dictionary_form AS BLOB)) > 0),
    PRIMARY KEY (command_id, token_index)
) STRICT;

CREATE TABLE "vocabulary_status_events" (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 1),
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    old_status TEXT CHECK (old_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    new_status TEXT NOT NULL CHECK (new_status IN ('new', 'learning', 'familiar', 'known', 'ignored')),
    cause TEXT NOT NULL CHECK (cause IN ('registration', 'manual', 'completion')),
    occurred_at_unix_ms INTEGER NOT NULL CHECK (occurred_at_unix_ms >= 0),
    completion_command_id INTEGER REFERENCES completion_analyses(command_id),
    CHECK ((cause = 'registration' AND old_status IS NULL AND new_status = 'new' AND completion_command_id IS NULL)
        OR (cause = 'manual' AND old_status IS NOT NULL AND old_status <> new_status AND completion_command_id IS NULL)
        OR (cause = 'completion' AND old_status IS NOT NULL AND old_status = 'new' AND new_status = 'known' AND completion_command_id IS NOT NULL))
) STRICT;


CREATE TABLE "sources" (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    kind TEXT NOT NULL CHECK (kind IN ('paste', 'txt', 'srt', 'epub')),
    original_title TEXT NOT NULL
);

CREATE TABLE "imported_files" (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    sha256 BLOB NOT NULL UNIQUE CHECK (length(sha256) = 32),
    original_bytes BLOB NOT NULL,
    text_encoding TEXT NOT NULL CHECK (text_encoding IN ('utf8', 'shift_jis', 'epub'))
) STRICT;

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
    kind TEXT NOT NULL CHECK (kind IN ('heading', 'emphasis', 'strong', 'external')),
    heading_level INTEGER,
    target TEXT,
    CHECK ((kind = 'heading' AND heading_level IS NOT NULL AND heading_level BETWEEN 1 AND 6 AND target IS NULL)
        OR (kind IN ('emphasis', 'strong') AND heading_level IS NULL AND target IS NULL)
        OR (kind = 'external' AND heading_level IS NULL AND target IS NOT NULL)),
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

CREATE TABLE effective_completion_commands (
    command_id INTEGER PRIMARY KEY REFERENCES completion_commands(id),
    semantics_version INTEGER NOT NULL CHECK (semantics_version = 2),
    payload TEXT NOT NULL CHECK (json_valid(payload))
) STRICT;

CREATE TABLE library_read_clock (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    sequence INTEGER NOT NULL CHECK (typeof(sequence) = 'integer' AND sequence >= 0)
);

CREATE TABLE anki_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    default_deck_id INTEGER CHECK (default_deck_id > 0)
) STRICT;

CREATE INDEX reading_completions_by_date ON reading_completions(completed_at_unix_ms, command_id);

CREATE INDEX paste_content_hashes_by_digest ON paste_content_hashes(sha256, document_id);

CREATE INDEX reading_occurrences_by_entry ON reading_occurrences(vocabulary_id, command_id);

CREATE INDEX vocabulary_status_events_by_date ON vocabulary_status_events(occurred_at_unix_ms, id);

CREATE INDEX vocabulary_status_events_by_entry ON vocabulary_status_events(vocabulary_id, id);

CREATE UNIQUE INDEX vocabulary_completion_status_once
    ON vocabulary_status_events(completion_command_id, vocabulary_id) WHERE cause = 'completion';

INSERT INTO reader_settings VALUES (1, 250, 16, 150);
INSERT INTO library_read_clock VALUES (1, 0);
INSERT INTO anki_settings VALUES (1, NULL);

CREATE TABLE statistics_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    timeframe TEXT NOT NULL CHECK (timeframe IN ('last_week', 'last_month', 'last_six_months', 'last_year', 'all_time'))
) STRICT;
INSERT INTO statistics_settings (id, timeframe) VALUES (1, 'last_week');
