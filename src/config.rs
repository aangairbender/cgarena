use serde::{Deserialize, Serialize};

use crate::{models::Language, services::exec};

pub const CONFIG_FILE: &str = "config.toml";

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub game: GameConfig,
    pub languages: Vec<Language>,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: i32,
    pub max_players: i32,
    pub symmetric: bool,
    pub referee: String,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            min_players: 2,
            max_players: 2,
            symmetric: true,
            referee: "".to_string(),
        }
    }
}

impl Config {
    pub fn open() -> Self {
        let config_path = std::env::current_dir().unwrap().join(CONFIG_FILE);
        let config_content =
            std::fs::read_to_string(config_path).expect("Config file should be available");

        let config: Config = toml::from_str(&config_content).expect("Config should be valid");

        for lang in &config.languages {
            if !exec::health_check(lang) {
                panic!("Language '{}' didn't pass the health check", &lang.name);
            }
        }
        config
    }
}
