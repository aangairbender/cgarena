CREATE TABLE match_attribute_names
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE
);

CREATE TABLE match_attribute_string_values
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    value       TEXT    NOT NULL UNIQUE
);

CREATE TABLE match_attributes
(
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    name_id          INTEGER NOT NULL,
    match_id         INTEGER NOT NULL,
    bot_id           INTEGER,
    turn             INTEGER,
    value_int        INTEGER,
    value_float      REAL,
    value_string_id  INTEGER,
    
    FOREIGN KEY (name_id) REFERENCES match_attribute_names (id),
    FOREIGN KEY (value_string_id) REFERENCES match_attribute_string_values (id),
    FOREIGN KEY (match_id) REFERENCES matches (id) ON DELETE CASCADE,
    FOREIGN KEY (bot_id) REFERENCES bots (id) ON DELETE CASCADE
);