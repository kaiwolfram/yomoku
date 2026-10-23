CREATE TABLE reader_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    page_length INTEGER NOT NULL CHECK (page_length IN (250, 500, 750, 1000, 1500))
) STRICT;

INSERT INTO reader_settings (id, page_length) VALUES (1, 500);
