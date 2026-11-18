CREATE INDEX vocabulary_by_registration ON vocabulary (registered_at_unix_ms, id);
CREATE INDEX epub_inline_images_by_asset ON epub_inline_images (document_id, path);
