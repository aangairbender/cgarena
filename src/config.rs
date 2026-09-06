use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{
    matchmaking::MatchmakingAlgorithmConfig,
    ranking::algorithms::{bradley_terry, elo, openskill, trueskill},
};

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

#[derive(Clone, Serialize, Deserialize)]
pub struct ArenaConfig {
    pub game: GameConfig,
    pub matchmaking: MatchmakingConfig,
    pub ranking: RankingConfig,
    #[serde(default)]
    pub leaderboards: LeaderboardsConfig,
    pub workers: Vec<WorkerConfig>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BootstrapConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub min_players: u32,
    pub max_players: u32,
    pub symmetric: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MatchmakingConfig {
    #[serde(flatten)]
    pub algorithm: MatchmakingAlgorithmConfig,
    pub enabled_on_start: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "algorithm")]
pub enum RankingConfig {
    OpenSkill(openskill::Config),
    TrueSkill(trueskill::Config),
    Elo(elo::Config),
    BradleyTerry(bradley_terry::Config),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum WorkerConfig {
    Embedded(EmbeddedWorkerConfig),
    // Remote
}

#[derive(Serialize, Clone)]
pub struct EmbeddedWorkerConfig {
    pub threads: u8,
    pub referee: RefereeConfig,
    pub cmd_build: String,
    pub cmd_run: String,
}

#[derive(Deserialize)]
struct RawEmbeddedWorkerConfig {
    threads: u8,
    referee: Option<RefereeConfig>,
    cmd_play_match: Option<String>,
    cmd_watch_replay: Option<String>,
    cmd_build: String,
    cmd_run: String,
}

impl<'de> Deserialize<'de> for EmbeddedWorkerConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = RawEmbeddedWorkerConfig::deserialize(deserializer)?;
        let referee = match (raw.referee, raw.cmd_play_match, raw.cmd_watch_replay) {
            (Some(referee), None, None) => referee,
            (None, Some(play_match), Some(watch_replay)) => {
                RefereeConfig::Command(CommandRefereeConfig {
                    play_match,
                    watch_replay,
                    legacy: true,
                })
            }
            (Some(_), _, _) => {
                return Err(serde::de::Error::custom(
                    "referee cannot be combined with legacy cmd_play_match or cmd_watch_replay",
                ))
            }
            (None, _, _) => {
                return Err(serde::de::Error::custom(
                    "configure referee or both legacy cmd_play_match and cmd_watch_replay",
                ))
            }
        };
        Ok(Self {
            threads: raw.threads,
            referee,
            cmd_build: raw.cmd_build,
            cmd_run: raw.cmd_run,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RefereeConfig {
    ManagedCodingame(ManagedCodingameRefereeConfig),
    Command(CommandRefereeConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManagedCodingameRefereeConfig {
    pub repository_url: String,
    pub branch: Option<String>,
    pub java: Option<String>,
    pub maven: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommandRefereeConfig {
    pub play_match: String,
    pub watch_replay: String,
    #[serde(skip)]
    pub(crate) legacy: bool,
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

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LeaderboardsConfig {
    pub uncertainty_coefficient: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG_CONTENT).unwrap()
    }
}
impl From<Config> for ArenaConfig {
    fn from(config: Config) -> Self {
        Self {
            game: config.game,
            matchmaking: config.matchmaking,
            ranking: config.ranking,
            leaderboards: config.leaderboards,
            workers: config.workers,
        }
    }
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Config::default().into()
    }
}

impl Config {
    pub fn load_legacy(arena_path: &Path) -> Result<Option<Config>, anyhow::Error> {
        let config_content = read_config_file(arena_path)?;
        let value: toml::Value =
            toml::from_str(&config_content).context("Config file format should be a valid TOML")?;
        if value.get("game").is_none() {
            return Ok(None);
        }
        value
            .try_into()
            .map(Some)
            .context("Config file format should be a valid arena configuration")
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        validate_arena_config(
            &self.game,
            &self.matchmaking,
            &self.ranking,
            &self.leaderboards,
            &self.workers,
        )
    }
}

impl ArenaConfig {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        validate_arena_config(
            &self.game,
            &self.matchmaking,
            &self.ranking,
            &self.leaderboards,
            &self.workers,
        )
    }
}

impl BootstrapConfig {
    pub fn load(arena_path: &Path) -> Result<Self, anyhow::Error> {
        let config_content = read_config_file(arena_path)?;
        toml::from_str(&config_content)
            .context("Config file format should contain valid server and logging settings")
    }
}

fn read_config_file(arena_path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(arena_path.join(CONFIG_FILE_NAME)).context("Cannot open config file")
}

fn validate_arena_config(
    game: &GameConfig,
    _matchmaking: &MatchmakingConfig,
    _ranking: &RankingConfig,
    _leaderboards: &LeaderboardsConfig,
    workers: &[WorkerConfig],
) -> Result<(), anyhow::Error> {
    if game.min_players == 0 {
        bail!("game.min_players must be at least 1");
    }
    if game.max_players > 8 {
        bail!("Games with up to 8 players are supported");
    }
    if game.min_players > game.max_players {
        bail!("game.max_players must be not less than game.min_players");
    }
    if workers.len() != 1 {
        bail!("exactly one embedded worker must be configured");
    }
    for config in workers {
        let WorkerConfig::Embedded(config) = config;
        if config.threads == 0 {
            bail!("embedded worker must have at least one thread");
        }

        if config.cmd_build.split_ascii_whitespace().count() == 0 {
            bail!("cmd_build must not be blank");
        }
        if config.cmd_run.split_ascii_whitespace().count() == 0 {
            bail!("cmd_run must not be blank");
        }
        match &config.referee {
            RefereeConfig::ManagedCodingame(config) => {
                if config.repository_url.trim().is_empty() {
                    bail!("managed CodinGame repository URL must not be blank");
                }
                for (name, value) in [
                    ("branch", config.branch.as_deref()),
                    ("java", config.java.as_deref()),
                    ("maven", config.maven.as_deref()),
                ] {
                    if value.is_some_and(|value| value.trim().is_empty()) {
                        bail!("managed CodinGame repository {name} must not be blank");
                    }
                }
                if config
                    .branch
                    .as_deref()
                    .is_some_and(|branch| branch.starts_with('-'))
                {
                    bail!("managed CodinGame repository branch must not begin with '-'");
                }
            }

            RefereeConfig::Command(config) => {
                if config.play_match.split_ascii_whitespace().count() == 0 {
                    bail!("command referee play_match must not be blank");
                }
                if config.watch_replay.split_ascii_whitespace().count() == 0 {
                    bail!("command referee watch_replay must not be blank");
                }
                if !config.legacy {
                    let play_match = shell_words::split(&config.play_match)
                        .context("command referee play_match has invalid quoting")?;
                    let watch_replay = shell_words::split(&config.watch_replay)
                        .context("command referee watch_replay has invalid quoting")?;
                    require_placeholder(&play_match, "{SEED}", "play_match")?;
                    require_placeholder(&play_match, "{REPLAY_PATH}", "play_match")?;
                    if !play_match.iter().any(|part| part == "{PLAYERS}") {
                        for player in 1..=game.max_players {
                            require_placeholder(
                                &play_match,
                                &format!("{{P{player}}}"),
                                "play_match",
                            )?;
                        }
                    }
                    for placeholder in ["{REPLAY_PATH}", "{REPLAY_DIR}", "{PORT}", "{PLAYER_COUNT}"]
                    {
                        require_placeholder(&watch_replay, placeholder, "watch_replay")?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn require_placeholder(template: &[String], placeholder: &str, field: &str) -> anyhow::Result<()> {
    if !template.iter().any(|part| part == placeholder) {
        bail!("command referee {field} must contain {placeholder}");
    }
    Ok(())
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
        let config: Config = toml::from_str(DEFAULT_CONFIG_CONTENT).expect("to be a valid config");
        config.validate().expect("default config should validate");
    }

    #[test]
    fn legacy_match_commands_migrate_to_command_referee() {
        let config: EmbeddedWorkerConfig = toml::from_str(
            r#"threads = 1
cmd_play_match = "runner {SEED} {PLAYERS}"
cmd_watch_replay = "renderer {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}"
cmd_build = "build"
cmd_run = "run""#,
        )
        .unwrap();
        let RefereeConfig::Command(referee) = config.referee else {
            panic!("legacy command fields must migrate");
        };
        assert_eq!(referee.play_match, "runner {SEED} {PLAYERS}");
    }

    #[test]
    fn mixed_legacy_and_tagged_referee_configuration_is_rejected() {
        let result = toml::from_str::<EmbeddedWorkerConfig>(
            r#"threads = 1
cmd_play_match = "legacy"
cmd_watch_replay = "legacy"
cmd_build = "build"
cmd_run = "run"

[referee]
type = "command"
play_match = "new"
watch_replay = "new""#,
        );
        let Err(error) = result else {
            panic!("mixed referee configuration must be rejected");
        };

        assert!(error.to_string().contains("cannot be combined"));
    }

    #[test]
    fn tagged_command_referee_requires_complete_templates() {
        let mut config = Config::default();
        let [WorkerConfig::Embedded(worker)] = config.workers.as_mut_slice() else {
            panic!("default config must contain one embedded worker");
        };
        worker.referee = RefereeConfig::Command(CommandRefereeConfig {
            play_match: "runner {SEED} {REPLAY_PATH} {PLAYERS}".to_string(),
            watch_replay: "renderer {REPLAY_PATH}".to_string(),
            legacy: false,
        });

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("{REPLAY_DIR}"));
    }

    #[test]
    fn migrated_legacy_command_templates_keep_previous_validation_rules() {
        let legacy: EmbeddedWorkerConfig = toml::from_str(
            r#"threads = 1
cmd_play_match = "runner"
cmd_watch_replay = "renderer"
cmd_build = "build"
cmd_run = "run""#,
        )
        .unwrap();
        let mut config = Config::default();
        let [WorkerConfig::Embedded(worker)] = config.workers.as_mut_slice() else {
            panic!("default config must contain one embedded worker");
        };
        worker.referee = legacy.referee;

        config.validate().unwrap();
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

        let config: MatchmakingConfig =
            toml::from_str(toml_str).expect("Should parse V2 algorithm accurately");

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

        let config: MatchmakingConfig =
            toml::from_str(toml_str).expect("Should parse explicit V1 tag");

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
        assert!(
            result.is_err(),
            "Should fail because V2 is missing 'min_matches_per_pair'"
        );
    }
    #[test]
    fn zero_worker_threads_are_rejected() {
        let mut config = Config::default();
        let [WorkerConfig::Embedded(worker)] = config.workers.as_mut_slice() else {
            panic!("default config must contain one embedded worker");
        };
        worker.threads = 0;

        assert_eq!(
            config.validate().unwrap_err().to_string(),
            "embedded worker must have at least one thread"
        );
    }
}
