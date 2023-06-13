use serde::{Deserialize, Serialize};

use crate::models::Language;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub languages: Vec<Language>,
    pub server: ServerConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: i32,
    pub max_players: i32,
    pub symmetric: bool,
    pub referee_file: String,
    pub referee_cmd: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub embedded_worker_threads: u8,
}
