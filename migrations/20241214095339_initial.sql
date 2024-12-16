CREATE TABLE bots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_code TEXT NOT NULL,
    language TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(name)
);

CREATE TABLE bot_stats (
    bot_id INTEGER PRIMARY KEY,
    matches_played INTEGER NOT NULL,
    matches_with_error INTEGER NOT NULL,
    rating_mu DOUBLE NOT NULL,
    rating_sigma DOUBLE NOT NULL,
    FOREIGN KEY (bot_id) REFERENCES bots(id) ON DELETE CASCADE
)

CREATE TABLE matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    seed INTEGER NOT NULL,
);

CREATE TABLE participations (
    match_id INTEGER NOT NULL,
    bot_id INTEGER NOT NULL,
    `index` INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    error INTEGER NOT NULL,
    PRIMARY KEY (match_id, bot_id),
    FOREIGN KEY (match_id) REFERENCES matches(id) ON DELETE CASCADE,
    FOREIGN KEY (bot_id) REFERENCES bots(id) ON DELETE CASCADE
);

CREATE TABLE builds (
    bot_id INTEGER NOT NULL,
    worker_name TEXT NOT NULL,
    status INTEGER NOT NULL,
    error TEXT,
    PRIMARY KEY (bot_id, worker_name),
    FOREIGN KEY (bot_id) REFERENCES bots(id) ON DELETE CASCADE
);
