ALTER TABLE reader_settings ADD COLUMN furigana INTEGER NOT NULL DEFAULT 0
    CHECK (furigana IN (0, 1));
