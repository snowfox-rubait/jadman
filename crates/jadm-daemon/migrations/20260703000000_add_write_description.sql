-- Add write_description column to downloads table
ALTER TABLE downloads ADD COLUMN write_description BOOLEAN NOT NULL DEFAULT 0;
