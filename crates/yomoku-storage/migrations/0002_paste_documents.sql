-- Provenance is separate from live Library entries and their canonical text.
CREATE TABLE sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT CHECK (id > 0),
    kind TEXT NOT NULL CHECK (kind = 'paste'),
    original_title TEXT NOT NULL
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
);

-- A future document deletion removes this text and the Library entry while
-- retaining source/chapter identities for historical anchors.
CREATE TABLE chapter_texts (
    chapter_id INTEGER PRIMARY KEY REFERENCES chapters(id),
    text TEXT NOT NULL
);
