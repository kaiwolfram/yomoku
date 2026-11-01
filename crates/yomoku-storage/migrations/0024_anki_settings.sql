-- A preference, never a cached provider/deck validation or mutation permit.
-- No external IDs are chosen until the user configures a default deck.
CREATE TABLE anki_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    default_deck_id INTEGER CHECK (default_deck_id > 0)
) STRICT;

INSERT INTO anki_settings (id, default_deck_id) VALUES (1, NULL);
