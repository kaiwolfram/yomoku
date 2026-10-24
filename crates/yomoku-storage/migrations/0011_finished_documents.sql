-- This live-document marker is independent of current reading position.
-- Deleting/reimporting allocates a new document with the default false marker.
ALTER TABLE documents ADD COLUMN finished_at_least_once INTEGER NOT NULL DEFAULT 0
    CHECK (finished_at_least_once IN (0, 1));

-- Existing immutable EOF completions prove a paste was finished, even if its
-- current position has since moved backward. Position at EOF alone is no proof.
-- Only single-chapter pastes exist today; future formats need their own final-
-- chapter validation rather than treating any chapter's EOF as a book finish.
UPDATE documents SET finished_at_least_once = 1
WHERE id IN (
    SELECT r.document_id FROM reading_completions r
    JOIN documents d ON d.id = r.document_id AND d.source_id = r.source_id
    JOIN sources s ON s.id = d.source_id AND s.kind = 'paste'
    JOIN chapters c ON c.source_id = s.id AND c.chapter_index = 0
    JOIN chapter_texts t ON t.chapter_id = c.id
    WHERE r.chapter_id = c.id
        AND r.end_byte_offset = length(CAST(t.text AS BLOB))
);
