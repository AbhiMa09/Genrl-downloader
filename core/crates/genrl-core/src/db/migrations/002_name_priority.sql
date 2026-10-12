-- Phase 3: the order a show's names are searched in.
--
-- The design searches "once per name in priority order (romaji first, since
-- most fansubs use it), stopping as soon as a name returns enough good
-- candidates", so the name set has to remember what it tried first. Lower is
-- earlier: the main title is 0, English and Japanese follow, user aliases last.
ALTER TABLE show_names ADD COLUMN priority INTEGER NOT NULL DEFAULT 5;
CREATE INDEX idx_show_names_priority ON show_names(mal_id, priority);
