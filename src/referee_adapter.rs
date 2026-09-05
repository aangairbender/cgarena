use crate::config::{CodingameJarRefereeConfig, CommandRefereeConfig, RefereeConfig};
use anyhow::{bail, Context};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub const COMPATIBILITY_ARGUMENT: &str = "--cgarena-compat";
pub const COMPATIBILITY_VERSION: &str = "cgarena-referee-v1";
const COMPATIBILITY_TIMEOUT: Duration = Duration::from_secs(10);

pub enum RefereeAdapter<'a> {
    CodingameJar(&'a CodingameJarRefereeConfig),
    Command(&'a CommandRefereeConfig),
}

#[derive(Clone, Copy)]
pub enum MatchResultSource {
    CommandStdout,
    CodingameReplay,
}

pub struct PreparedMatchCommand {
    pub argv: Vec<String>,
    pub result_source: MatchResultSource,
}

pub struct PreparedReplayCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl<'a> From<&'a RefereeConfig> for RefereeAdapter<'a> {
    fn from(config: &'a RefereeConfig) -> Self {
        match config {
            RefereeConfig::CodingameJar(config) => Self::CodingameJar(config),
            RefereeConfig::Command(config) => Self::Command(config),
        }
    }
}

impl RefereeAdapter<'_> {
    pub fn match_replay_required(&self) -> bool {
        match self {
            Self::CodingameJar(_) => true,
            Self::Command(config) => config.play_match.contains("{REPLAY_PATH}"),
        }
    }

    pub fn prepare_match_command(
        &self,
        seed: i64,
        player_commands: &[String],
        replay_path: Option<&Path>,
    ) -> anyhow::Result<PreparedMatchCommand> {
        match self {
            Self::CodingameJar(config) => {
                let replay_path =
                    replay_path.context("codingame referee requires a replay path")?;
                let mut argv = vec![
                    config.java.as_deref().unwrap_or("java").to_string(),
                    "--add-opens".to_string(),
                    "java.base/java.lang=ALL-UNNAMED".to_string(),
                    "-jar".to_string(),
                    config.path.clone(),
                ];
                for (index, player_command) in player_commands.iter().enumerate() {
                    argv.push(format!("-p{}", index + 1));
                    argv.push(player_command.clone());
                }
                argv.extend([
                    "-seed".to_string(),
                    seed.to_string(),
                    "-league".to_string(),
                    config.league.unwrap_or(19).to_string(),
                    "-l".to_string(),
                    replay_path.to_string_lossy().to_string(),
                ]);
                Ok(PreparedMatchCommand {
                    argv,
                    result_source: MatchResultSource::CodingameReplay,
                })
            }
            Self::Command(config) => {
                let template = shell_words::split(&config.play_match)
                    .context("invalid command referee play_match quoting")?;
                let mut argv = Vec::new();
                for part in template {
                    match part.as_str() {
                        "{SEED}" => argv.push(seed.to_string()),
                        "{PLAYERS}" => argv.extend(player_commands.iter().cloned()),
                        "{REPLAY_PATH}" => argv.push(
                            replay_path
                                .context("command referee requires a replay path")?
                                .to_string_lossy()
                                .to_string(),
                        ),
                        _ => {
                            let player = part
                                .strip_prefix("{P")
                                .and_then(|value| value.strip_suffix('}'))
                                .and_then(|value| value.parse::<usize>().ok());
                            if let Some(player) = player {
                                argv.push(
                                    player_commands
                                        .get(player.saturating_sub(1))
                                        .with_context(|| {
                                            format!("command referee references missing {part}")
                                        })?
                                        .clone(),
                                );
                            } else {
                                argv.push(part);
                            }
                        }
                    }
                }
                if argv.is_empty() {
                    bail!("command referee play_match must not be blank");
                }
                Ok(PreparedMatchCommand {
                    argv,
                    result_source: MatchResultSource::CommandStdout,
                })
            }
        }
    }

    pub fn prepare_replay_command(
        &self,
        artifact_path: &Path,
        session_directory: &Path,
        temporary_directory: &Path,
        port: u16,
        participant_count: u8,
    ) -> anyhow::Result<PreparedReplayCommand> {
        match self {
            Self::CodingameJar(config) => Ok(PreparedReplayCommand {
                program: config.java.as_deref().unwrap_or("java").to_string(),
                args: vec![
                    format!("-Djava.io.tmpdir={}", temporary_directory.display()),
                    "--add-opens".to_string(),
                    "java.base/java.lang=ALL-UNNAMED".to_string(),
                    "-jar".to_string(),
                    config.path.clone(),
                    "-r".to_string(),
                    artifact_path.to_string_lossy().to_string(),
                    "-port".to_string(),
                    port.to_string(),
                ],
            }),
            Self::Command(config) => {
                let mut parts = shell_words::split(&config.watch_replay)
                    .context("invalid command referee watch_replay")?;
                if parts.is_empty() {
                    bail!("command referee watch_replay must not be blank");
                }
                let port = port.to_string();
                let participant_count = participant_count.to_string();
                let replacements = [
                    ("{REPLAY_PATH}", artifact_path.to_str()),
                    ("{REPLAY_DIR}", session_directory.to_str()),
                    ("{PORT}", Some(port.as_str())),
                    ("{PLAYER_COUNT}", Some(participant_count.as_str())),
                ];
                for (placeholder, value) in replacements {
                    let value =
                        value.with_context(|| format!("{placeholder} value is not valid UTF-8"))?;
                    let part = parts
                        .iter_mut()
                        .find(|part| part.as_str() == placeholder)
                        .with_context(|| {
                            format!("command referee watch_replay must contain {placeholder}")
                        })?;
                    *part = value.to_string();
                }
                Ok(PreparedReplayCommand {
                    program: parts.remove(0),
                    args: parts,
                })
            }
        }
    }

    pub async fn validate_startup(&self, arena_path: &Path) -> anyhow::Result<()> {
        let Self::CodingameJar(config) = self else {
            return Ok(());
        };
        let jar_path = resolve_path(arena_path, &config.path);
        let metadata = std::fs::metadata(&jar_path)
            .with_context(|| format!("cannot access referee JAR {}", jar_path.display()))?;
        if !metadata.is_file() {
            bail!("referee JAR is not a file: {}", jar_path.display());
        }
        std::fs::File::open(&jar_path)
            .with_context(|| format!("referee JAR is not readable: {}", jar_path.display()))?;

        let mut command = Command::new(config.java.as_deref().unwrap_or("java"));
        command
            .args(["--add-opens", "java.base/java.lang=ALL-UNNAMED", "-jar"])
            .arg(&jar_path)
            .arg(COMPATIBILITY_ARGUMENT)
            .current_dir(arena_path)
            .kill_on_drop(true);
        let output = timeout(COMPATIBILITY_TIMEOUT, command.output())
            .await
            .with_context(|| {
                format!(
                    "timed out validating referee JAR compatibility: {}",
                    jar_path.display()
                )
            })?
            .with_context(|| format!("cannot execute referee JAR {}", jar_path.display()))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success()
            || !stdout
                .lines()
                .any(|line| line.trim() == COMPATIBILITY_VERSION)
        {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let diagnostic = if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            };
            bail!(
                "unsupported referee JAR {} (expected {COMPATIBILITY_VERSION}, status {}): {}",
                jar_path.display(),
                output.status,
                diagnostic
            );
        }
        Ok(())
    }
}

fn resolve_path(arena_path: &Path, configured_path: &str) -> PathBuf {
    let path = Path::new(configured_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        arena_path.join(path)
    }
}

#[derive(Debug, Deserialize)]
pub struct MatchCommandOutput {
    #[serde(default)]
    pub scores: Vec<i32>,
    pub ranks: Vec<u8>,
    pub errors: Vec<u8>,
    #[serde(default)]
    pub attributes: Vec<MatchCommandAttribute>,
}

#[derive(Debug, Deserialize, Default)]
pub struct MatchCommandAttribute {
    pub name: String,
    pub player: Option<usize>,
    pub turn: Option<u16>,
    pub value: String,
}

pub fn read_codingame_match_result(
    path: &Path,
    player_count: usize,
) -> anyhow::Result<MatchCommandOutput> {
    let replay: Value = serde_json::from_slice(&std::fs::read(path)?)
        .context("codingame replay artifact must be valid JSON")?;
    let score_map = replay
        .get("scores")
        .and_then(Value::as_object)
        .context("codingame replay artifact has no scores object")?;
    if score_map.len() != player_count {
        bail!(
            "codingame replay has {} scores for {player_count} bots",
            score_map.len()
        );
    }
    let scores = (0..player_count)
        .map(|index| {
            score_map
                .get(&index.to_string())
                .and_then(Value::as_i64)
                .context("codingame replay score is not an integer")
                .and_then(|score| {
                    i32::try_from(score).context("codingame replay score is out of range")
                })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut attributes = Vec::new();
    if let Some(errors) = replay.get("errors").and_then(Value::as_object) {
        for player in 0..player_count {
            for line in errors
                .get(&player.to_string())
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .flat_map(str::lines)
            {
                if let Some(attribute) = parse_codingame_attribute(line, player) {
                    attributes.push(attribute);
                }
            }
        }
    }
    Ok(MatchCommandOutput {
        ranks: scores
            .iter()
            .map(|score| scores.iter().filter(|other| score < *other).count() as u8)
            .collect(),
        errors: scores.iter().map(|score| u8::from(*score < 0)).collect(),
        scores,
        attributes,
    })
}

pub fn validate_match_result(result: &MatchCommandOutput, bot_count: usize) -> anyhow::Result<()> {
    if result.ranks.len() != bot_count || result.errors.len() != bot_count {
        bail!("play match output must contain ranks and errors for {bot_count} bots");
    }
    if !result.scores.is_empty() && result.scores.len() != bot_count {
        bail!(
            "play match output has {} scores for {bot_count} bots",
            result.scores.len()
        );
    }
    if let Some(player) = result
        .attributes
        .iter()
        .filter_map(|attribute| attribute.player)
        .find(|player| *player >= bot_count)
    {
        bail!("play match output references missing player {player}");
    }
    Ok(())
}

fn parse_codingame_attribute(line: &str, player: usize) -> Option<MatchCommandAttribute> {
    let line = line.trim();
    let (owner, rest) = if let Some(rest) = strip_prefix_ignore_ascii_case(line, "[TDATA]") {
        (None, rest)
    } else if let Some(rest) = strip_prefix_ignore_ascii_case(line, "[PDATA]") {
        (Some(player), rest)
    } else {
        return None;
    };
    let rest = rest.trim_start();
    let (turn, rest) = if let Some(rest) = rest.strip_prefix('[') {
        let (turn, rest) = rest.split_once(']')?;
        (Some(turn.parse().ok()?), rest)
    } else {
        (None, rest)
    };
    let (name, value) = rest.trim().split_once('=')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    {
        return None;
    }
    Some(MatchCommandAttribute {
        name: name.to_string(),
        player: owner,
        turn,
        value: value.trim().to_string(),
    })
}

fn strip_prefix_ignore_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

pub async fn validate_codingame_replay(
    artifact_path: &Path,
    participant_count: u8,
) -> anyhow::Result<()> {
    let bytes = tokio::fs::read(artifact_path).await?;
    let replay: Value = serde_json::from_slice(&bytes)?;
    let agents = replay
        .get("agents")
        .and_then(Value::as_array)
        .context("artifact has no agents array")?;
    if agents.len() != usize::from(participant_count) {
        bail!(
            "artifact has {} participants, match has {participant_count}",
            agents.len()
        );
    }
    Ok(())
}

pub fn import_codingame_replay_bundle(source: &Path, destination: &Path) -> std::io::Result<()> {
    copy_directory(source, destination)?;
    normalize_replay_bundle(destination)
}

fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "renderer bundle contains a symbolic link",
            ));
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn normalize_replay_bundle(directory: &Path) -> std::io::Result<()> {
    let assets = directory.join("assets");
    if assets.is_dir() {
        let nested = assets.join("assets");
        std::fs::create_dir_all(&nested)?;
        for entry in std::fs::read_dir(&assets)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "png")
            {
                std::fs::copy(entry.path(), nested.join(entry.file_name()))?;
            }
        }
    }
    let app = directory.join("app.js");
    if app.is_file() {
        let content = std::fs::read_to_string(&app)?
            .replace("from '../config.js'", "from './config.js'")
            .replace("from '../demo.js'", "from './demo.js'")
            .replace(
                "viewerUrl: '/core/Drawer.js'",
                "viewerUrl: './core/Drawer.js'",
            );
        std::fs::write(app, content)?;
    }
    Ok(())
}
