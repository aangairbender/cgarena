CREATE TABLE bots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_code TEXT NOT NULL,
    language TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(name)
);

CREATE TABLE bot_stats (
    bot_id INTEGER NOT NULL,
    games INTEGER NOT NULL,
    rating_mu DOUBLE NOT NULL,
    rating_sigma DOUBLE NOT NULL,
    FOREIGN KEY(bot_id) REFERENCES bots(id) ON DELETE CASCADE
);

CREATE TABLE matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    seed INTEGER NOT NULL
);

CREATE TABLE participations (
    match_id INTEGER NOT NULL,
    bot_id INTEGER NOT NULL,
    `index` INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    error INTEGER NOT NULL,
    FOREIGN KEY(match_id) REFERENCES matches(id) ON DELETE CASCADE,
    FOREIGN KEY(bot_id) REFERENCES bots(id) ON DELETE CASCADE
);

CREATE TABLE builds (
    bot_id INTEGER NOT NULL,
    worker_name INTEGER NOT NULL,
    status_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (bot_id, worker_name),
    FOREIGN KEY (bot_id) REFERENCES bots(id) ON DELETE CASCADE
);
