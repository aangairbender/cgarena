use std::{fs::OpenOptions, io::Write, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub matchmaking: MatchmakingConfig,
    pub ranking: RankingConfig,
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
pub struct MatchmakingConfig {
    pub allow_same_bots: bool,
    pub min_matches: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "algorithm")]
#[serde(rename_all = "snake_case")]
pub enum RankingConfig {
    WengLin,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerConfig {
    pub threads: u8,
    pub workdir: String,
    pub play_match_command: String,
    pub languages: Vec<LanguageConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct LanguageConfig {
    pub name: String,
    pub build_command: String,
    pub run_command: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

impl Config {
    pub fn load(arena_path: &Path) -> Result<Config, anyhow::Error> {
        let path = arena_path.join(CONFIG_FILE_NAME);
        let config_content = std::fs::read_to_string(path)?;
        let config = serde_yaml::from_str(&config_content)?;
        Ok(config)
    }

    pub fn create_default(arena_path: &Path) -> Result<(), std::io::Error> {
        let config_file_path = arena_path.join(CONFIG_FILE_NAME);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(config_file_path)?;
        file.write_all(DEFAULT_CONFIG_CONTENT.as_bytes())?;
        Ok(())
    }
}

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cgarena_config.yaml"
));

const CONFIG_FILE_NAME: &str = "cgarena_config.yaml";

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let _: Config = serde_yaml::from_str(DEFAULT_CONFIG_CONTENT).expect("to be a valid config");
    }
}
