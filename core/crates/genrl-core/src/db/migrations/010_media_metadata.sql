-- Extend media table with additional metadata fields
ALTER TABLE media ADD COLUMN original_title TEXT;
ALTER TABLE media ADD COLUMN poster_path TEXT;
ALTER TABLE media ADD COLUMN status TEXT;
ALTER TABLE media ADD COLUMN total_seasons INTEGER;
ALTER TABLE media ADD COLUMN total_episodes INTEGER;
ALTER TABLE media ADD COLUMN raw_json TEXT;
ALTER TABLE media ADD COLUMN updated_at INTEGER;
