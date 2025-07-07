CREATE TABLE match_attributes
(
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    match_id    INTEGER NOT NULL,
    bot_id      INTEGER,
    turn        INTEGER,
    value       TEXT    NOT NULL,
    
    FOREIGN KEY (match_id) REFERENCES matches (id) ON DELETE CASCADE,
    FOREIGN KEY (bot_id) REFERENCES bots (id) ON DELETE CASCADE
);