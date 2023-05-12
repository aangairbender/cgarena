use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &'static str = "config.toml";

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub game: GameConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub title: String,
    pub min_players: i32,
    pub max_players: i32,
    pub symmetric: bool,
    pub referee: String,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "CG game".to_string(),
            min_players: 2,
            max_players: 2,
            symmetric: true,
            referee: "".to_string(),
        }
    }
}


impl Config {
    pub fn open() -> Self {
        let config_path = std::env::current_dir()
            .unwrap()
            .join(CONFIG_FILE);
        let config_content = std::fs::read_to_string(config_path)
            .expect("Config file should be available");
        
        toml::from_str(&config_content)
            .expect("Config should be valid")
    }
}
