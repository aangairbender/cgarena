CREATE TABLE arena_configuration (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    config_json TEXT NOT NULL
);
