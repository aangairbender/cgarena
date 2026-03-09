use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{matchmaking::MatchmakingAlgorithmConfig, ranking::algorithms::{bradley_terry, elo, openskill, trueskill}};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub game: GameConfig,
    pub matchmaking: MatchmakingConfig,
    pub ranking: RankingConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub leaderboards: LeaderboardsConfig,
    pub workers: Vec<WorkerConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: u32,
    pub max_players: u32,
    pub symmetric: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MatchmakingConfig {
    #[serde(flatten)]
    pub algorithm: MatchmakingAlgorithmConfig,
    pub enabled_on_start: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "algorithm")]
pub enum RankingConfig {
    OpenSkill(openskill::Config),
    TrueSkill(trueskill::Config),
    Elo(elo::Config),
    BradleyTerry(bradley_terry::Config),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum WorkerConfig {
    Embedded(EmbeddedWorkerConfig),
    // Remote
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EmbeddedWorkerConfig {
    pub threads: u8,
    pub cmd_play_match: String,
    pub cmd_build: String,
    pub cmd_run: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ServerConfig {
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub expose: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LogConfig {
    pub level: Option<String>,
    pub file: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct LeaderboardsConfig {
    pub uncertainty_coefficient: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG_CONTENT).unwrap()
    }
}

impl Config {
    pub fn load(arena_path: &Path) -> Result<Config, anyhow::Error> {
        let path = arena_path.join(CONFIG_FILE_NAME);
        let config_content = std::fs::read_to_string(path).context("Cannot open config file")?;
        let config: Config =
            toml::from_str(&config_content).context("Config file format should be a valid TOML")?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.game.max_players > 8 {
            bail!("Games with up to 8 players are supported");
        }
        if self.game.min_players > self.game.max_players {
            bail!("game.max_players must be not less than game.min_players");
        }
        for config in &self.workers {
            let WorkerConfig::Embedded(config) = config;

            if config.cmd_build.split_ascii_whitespace().count() == 0 {
                bail!("cmd_build must not be blank");
            }
            if config.cmd_run.split_ascii_whitespace().count() == 0 {
                bail!("cmd_run must not be blank");
            }
            if config.cmd_play_match.split_ascii_whitespace().count() == 0 {
                bail!("cmd_play_match must not be blank");
            }
        }
        Ok(())
    }
}

const CONFIG_FILE_NAME: &str = "cgarena_config.toml";

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/default_config.toml"
));

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let _: Config = toml::from_str(DEFAULT_CONFIG_CONTENT).expect("to be a valid config");
    }

    #[test]
    fn test_matchmaking_legacy_fallback_no_tag() {
        // Old config file: No "algorithm" key exists
        let toml_str = r#"
            enabled_on_start = true
            min_matches = 10
            min_matches_preference = 0.5
        "#;

        let config: MatchmakingConfig = toml::from_str(toml_str)
            .expect("Should parse legacy config by falling back to Legacy variant");

        // Verify it mapped to the Legacy variant containing V1 data
        match config.algorithm {
            MatchmakingAlgorithmConfig::Legacy(v1) => {
                assert_eq!(v1.min_matches, 10);
                assert_eq!(v1.min_matches_preference, 0.5);
            }
            _ => panic!("Expected Legacy variant for missing tag"),
        }
    }

    #[test]
    fn test_matchmaking_explicit_v2_tag() {
        // New config file: Explicitly using the "v2" algorithm
        let toml_str = r#"
            algorithm = "v2"
            enabled_on_start = true
            min_matches_per_pair = 20
        "#;

        let config: MatchmakingConfig = toml::from_str(toml_str)
            .expect("Should parse V2 algorithm accurately");

        match config.algorithm {
            MatchmakingAlgorithmConfig::V2(v2) => {
                assert_eq!(v2.min_matches_per_pair, 20);
                assert!(v2.max_matches.is_none());
            }
            _ => panic!("Expected V2 variant"),
        }
    }

    #[test]
    fn test_matchmaking_explicit_v1_tag() {
        // User explicitly wants V1 by name
        let toml_str = r#"
            algorithm = "v1"
            min_matches = 5
            min_matches_preference = 0.1
        "#;

        let config: MatchmakingConfig = toml::from_str(toml_str)
            .expect("Should parse explicit V1 tag");

        match config.algorithm {
            MatchmakingAlgorithmConfig::V1(v1) => {
                assert_eq!(v1.min_matches, 5);
            }
            _ => panic!("Expected V1 variant"),
        }
    }

    #[test]
    fn test_matchmaking_v2_missing_required_field() {
        // When a tag is provided, Serde becomes strict.
        // If "algorithm = v2" is set, but required fields are missing, it should fail.
        let toml_str = r#"
            algorithm = "v2"
            enabled_on_start = true
        "#;

        let result: Result<MatchmakingConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err(), "Should fail because V2 is missing 'min_matches_per_pair'");
    }
}
