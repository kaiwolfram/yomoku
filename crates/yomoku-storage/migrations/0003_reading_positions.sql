CREATE TABLE reading_positions (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    chapter_id INTEGER NOT NULL REFERENCES chapters(id),
    byte_offset INTEGER NOT NULL CHECK (byte_offset >= 0)
);
