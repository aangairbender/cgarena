ALTER TABLE matches ADD COLUMN replay_watched_at INTEGER;

UPDATE matches
SET replay_watched_at = unixepoch();
