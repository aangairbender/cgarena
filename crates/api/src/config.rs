use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub server: ServerConfig,
    pub embedded_worker: Option<WorkerConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: u32,
    pub max_players: u32,
    pub symmetric: bool,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerConfig {
    pub threads: u8,
    pub workdir: String,
    pub language_templates_path: String,
    pub referee_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, anyhow::Error> {
        let config_content = std::fs::read_to_string(path)?;
        let config = toml::from_str(&config_content)?;
        Ok(config)
    }
}
