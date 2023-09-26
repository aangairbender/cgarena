CREATE TABLE IF NOT EXISTS bots (
    id text primary key,
    name text not null,
    language text not null,
    source_filename text not null,
    status text not null,
    build_output text not null
);
