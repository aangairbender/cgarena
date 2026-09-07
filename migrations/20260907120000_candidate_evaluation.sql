CREATE TABLE evaluation_plan_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    config_json TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE evaluation_stage_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_revision_id INTEGER NOT NULL,
    stage_index INTEGER NOT NULL,
    config_json TEXT NOT NULL,
    FOREIGN KEY (plan_revision_id) REFERENCES evaluation_plan_revisions(id),
    UNIQUE (plan_revision_id, stage_index)
);

ALTER TABLE bots ADD COLUMN role TEXT NOT NULL DEFAULT 'benchmark'
    CHECK (role IN ('candidate', 'benchmark', 'archived_benchmark'));
ALTER TABLE bots ADD COLUMN evaluation_plan_revision_id INTEGER
    REFERENCES evaluation_plan_revisions(id);

ALTER TABLE matches ADD COLUMN candidate_bot_id INTEGER REFERENCES bots(id);
ALTER TABLE matches ADD COLUMN evaluation_stage_revision_id INTEGER
    REFERENCES evaluation_stage_revisions(id);

CREATE INDEX matches_evaluation_stage_candidate_idx
    ON matches(evaluation_stage_revision_id, candidate_bot_id);
