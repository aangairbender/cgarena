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
    FOREIGN KEY(bot_id) REFERENCES bots(id)
);

CREATE TABLE matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    seed INTEGER NOT NULL
);

CREATE TABLE participations (
    match_id INTEGER NOT NULL,
    bot_id INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    error INTEGER NOT NULL,
    FOREIGN KEY(match_id) REFERENCES matches(id),
    FOREIGN KEY(bot_id) REFERENCES bots(id)
);
