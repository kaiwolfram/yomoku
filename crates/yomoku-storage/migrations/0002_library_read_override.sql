ALTER TABLE documents ADD COLUMN read_override INTEGER CHECK (read_override IN (0, 1));
