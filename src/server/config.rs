use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub server: ServerConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: i32,
    pub max_players: i32,
    pub symmetric: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub embedded_worker_threads: u8,
    pub worker_template_path: String,
    pub referee_path: String,
    pub referee_cmd: String,
}
