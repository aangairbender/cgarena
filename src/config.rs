use serde::{Deserialize, Serialize};

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
