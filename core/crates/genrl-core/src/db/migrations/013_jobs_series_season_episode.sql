-- Add season and episode columns to jobs for series downloads
ALTER TABLE jobs ADD COLUMN season INTEGER;
ALTER TABLE jobs ADD COLUMN episode INTEGER;
