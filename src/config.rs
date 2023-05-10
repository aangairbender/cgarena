use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub arena: ArenaConfig,
    pub game: GameConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ArenaConfig {
    pub port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub title: String,
    pub min_players: i32,
    pub max_players: i32,
    pub symmetric: bool,
    pub referee: String,
}
