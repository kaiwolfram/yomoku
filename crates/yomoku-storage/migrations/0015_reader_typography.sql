ALTER TABLE reader_settings ADD COLUMN font_size INTEGER NOT NULL DEFAULT 16
    CHECK (font_size IN (14, 16, 18, 20, 24));
ALTER TABLE reader_settings ADD COLUMN line_spacing INTEGER NOT NULL DEFAULT 150
    CHECK (line_spacing IN (125, 150, 175, 200));
