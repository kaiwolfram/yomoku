CREATE TABLE reader_word_markers (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    chapter_id INTEGER NOT NULL REFERENCES chapters(id),
    start_byte INTEGER NOT NULL CHECK (start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK (end_byte > start_byte)
) STRICT;
