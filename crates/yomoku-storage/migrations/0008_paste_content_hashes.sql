-- Identity for pasted canonical UTF-8 bytes only, not extracted file content.
-- Keep preexisting duplicate documents intact; imports reference the oldest live
-- match. Missing hashes are filled lazily by the import repository, not at open.
CREATE TABLE paste_content_hashes (
    document_id INTEGER PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    sha256 BLOB NOT NULL CHECK (length(sha256) = 32)
) STRICT;

CREATE INDEX paste_content_hashes_by_digest ON paste_content_hashes(sha256, document_id);
