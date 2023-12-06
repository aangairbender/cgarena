CREATE TABLE IF NOT EXISTS bots (
    id text primary key,
    name text not null,
    language text not null,
    source_filename text not null,
    created_at timestamp not null
);

CREATE TABLE IF NOT EXISTS matches (
    id text primary key,
    seed integer not null,
    status text not null,
    created_at timestamp not null
);

CREATE TABLE IF NOT EXISTS participations (
    match_id text not null,
    bot_id text not null,
    `index`: integer not null,
    score: integer,
    PRIMARY KEY(match_id, bot_id, `index`),
    FOREIGN KEY(match_id) REFERENCES matches(id),
    FOREIGN KEY(bot_id) REFERENCES bots(id)
);
