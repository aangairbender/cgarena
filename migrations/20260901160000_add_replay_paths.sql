ALTER TABLE matches ADD COLUMN replay_path TEXT;

CREATE UNIQUE INDEX matches_replay_path_unique
    ON matches (replay_path)
    WHERE replay_path IS NOT NULL;
