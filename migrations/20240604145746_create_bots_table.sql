CREATE TABLE bots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_code TEXT NOT NULL,
    language TEXT NOT NULL,
    status TEXT NOT NULL,
    rating_mu DOUBLE NOT NULL,
    rating_sigma DOUBLE NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(name)
)