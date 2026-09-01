use crate::config::EmbeddedWorkerConfig;
use crate::domain::{
    BotId, BuildResult, Language, MatchAttribute, MatchAttributeValue, Participant, SourceCode,
    WorkerName,
};
use anyhow::{bail, Context};
use itertools::Itertools;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{oneshot, Semaphore};
use tokio::{fs, process::Command};
use tokio_util::sync::CancellationToken;

pub struct WorkerHandle {
    pub match_tx: Sender<PlayMatchInput>,
    pub match_result_rx: Receiver<PlayMatchOutput>,
    pub build_tx: Sender<BuildCmd>,
    pub known_bot_ids: Vec<BotId>,
}

impl WorkerHandle {
    pub async fn build_bot(&self, input: BuildBotInput) -> BuildBotOutput {
        let (tx, rx) = oneshot::channel();
        let cmd = BuildCmd { input, result: tx };
        let _ = self.build_tx.send(cmd).await;
        rx.await.unwrap()
    }
}

pub struct BuildCmd {
    pub input: BuildBotInput,
    pub result: oneshot::Sender<BuildBotOutput>,
}

const DIR_BOTS: &str = "bots";

pub fn run_embedded_worker(
    worker_path: &Path,
    config: EmbeddedWorkerConfig,
) -> anyhow::Result<WorkerHandle> {
    let config = Arc::new(config);

    let known_bot_ids = known_bot_ids(worker_path)?;

    let (match_result_tx, match_result_rx) = channel(100);
    let (match_tx, match_rx) = channel(config.threads as usize * 2);
    tokio::spawn(run_play_matches(
        match_rx,
        worker_path.to_path_buf(),
        Arc::clone(&config),
        match_result_tx,
    ));

    let (build_tx, build_rx) = channel(1);
    tokio::spawn(run_build_bots(worker_path.to_path_buf(), config, build_rx));

    let handle = WorkerHandle {
        match_tx,
        match_result_rx,
        build_tx,
        known_bot_ids,
    };
    Ok(handle)
}

fn known_bot_ids(worker_path: &Path) -> anyhow::Result<Vec<BotId>> {
    let bots_folder = worker_path.join(DIR_BOTS);
    let mut res = vec![];

    if !bots_folder.exists() {
        return Ok(vec![]);
    }

    for entry in std::fs::read_dir(bots_folder)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let Ok(bot_id_i64) = name.parse::<i64>() else {
            continue;
        };

        res.push(BotId::from(bot_id_i64));
    }

    Ok(res)
}

async fn run_build_bots(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    mut rx: Receiver<BuildCmd>,
) {
    while let Some(cmd) = rx.recv().await {
        let bot_id = cmd.input.bot_id;
        let worker_name = cmd.input.worker_name.clone();

        let result = build_bot(worker_path.clone(), Arc::clone(&config), cmd.input)
            .await
            .unwrap_or_else(|e| BuildResult::Failure {
                stderr: e.to_string(),
            });

        let output = BuildBotOutput {
            bot_id,
            worker_name,
            result,
        };
        let _ = cmd.result.send(output);
    }
}

async fn build_bot(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    input: BuildBotInput,
) -> anyhow::Result<BuildResult> {
    let bot_folder_relative = PathBuf::from(DIR_BOTS).join(i64::from(input.bot_id).to_string());
    let bot_folder = worker_path.join(&bot_folder_relative);

    fs::create_dir_all(&bot_folder)
        .await
        .context("Failed to create bot folder")?;
    fs::write(
        bot_folder.join("source.txt"),
        &String::from(input.source_code),
    )
    .await
    .context("Cannot create source.txt file")?;

    let dir_param_value = bot_folder_relative
        .to_str()
        .context("Bot folder path is not utf-8")?;
    let command_parts = config
        .cmd_build
        .replace("{DIR}", dir_param_value)
        .replace("{LANG}", &input.language)
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect_vec();
    assert_ne!(command_parts.len(), 0);

    let output = Command::new(&command_parts[0])
        .args(&command_parts[1..])
        .current_dir(&worker_path)
        .output()
        .await
        .context("Failed to execute command")?;

    let res = if output.status.success() {
        BuildResult::Success
    } else {
        BuildResult::Failure {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    };
    Ok(res)
}

async fn run_play_matches(
    mut rx: Receiver<PlayMatchInput>,
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    match_result_tx: Sender<PlayMatchOutput>,
) {
    let semaphore = Arc::new(Semaphore::new(config.threads as usize));
    let token = CancellationToken::new();

    while let Some(input) = rx.recv().await {
        if token.is_cancelled() {
            break;
        }

        let (command_parts, replay_path) = match prepare_play_match_command(&config, &input) {
            Ok(command) => command,
            Err(error) => {
                token.cancel();
                tracing::error!("{error:#}");
                break;
            }
        };

        let semaphore = Arc::clone(&semaphore);
        let permit = semaphore.acquire_owned().await.expect("Semaphore poisoned");
        let match_result_tx_clone = match_result_tx.clone();
        let worker_path_clone = worker_path.clone();
        let token_clone = token.clone();
        tokio::spawn(async move {
            let res =
                spawn_play_match_command(command_parts, replay_path, worker_path_clone, input)
                    .await;

            match res {
                Ok(output) => {
                    let _ = match_result_tx_clone.send(output).await;
                }
                Err(e) => {
                    token_clone.cancel(); // this should make cgarena stop eventually
                    tracing::error!("{}", e);
                }
            }
            drop(permit);
        });
    }
}

fn prepare_play_match_command(
    config: &EmbeddedWorkerConfig,
    input: &PlayMatchInput,
) -> anyhow::Result<(Vec<String>, Option<PathBuf>)> {
    let run_commands = input
        .bots
        .iter()
        .map(|bot| {
            let bot_folder_relative =
                PathBuf::from(DIR_BOTS).join(i64::from(bot.bot_id).to_string());
            let dir_param_value = bot_folder_relative
                .to_str()
                .context("bot folder must be utf-8")?;
            Ok(config
                .cmd_run
                .replace("{DIR}", dir_param_value)
                .replace("{LANG}", &bot.language))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let template =
        shell_words::split(&config.cmd_play_match).context("invalid cmd_play_match quoting")?;
    if template.is_empty() {
        bail!("cmd_play_match must not be blank");
    }

    let replay_path = template
        .iter()
        .any(|part| part == "{REPLAY_PATH}")
        .then(crate::replay_artifact::allocate);
    let replay_path_value = replay_path
        .as_ref()
        .and_then(|path| path.to_str())
        .context("replay path must be UTF-8")
        .or_else(|error| {
            if replay_path.is_none() {
                Ok("")
            } else {
                Err(error)
            }
        })?;
    let seed = input.seed.to_string();
    let mut command_parts = Vec::new();

    for part in template {
        match part.as_str() {
            "{SEED}" => command_parts.push(seed.clone()),
            "{PLAYERS}" => command_parts.extend(run_commands.iter().cloned()),
            "{REPLAY_PATH}" => command_parts.push(replay_path_value.to_string()),
            _ => {
                let player_index = part
                    .strip_prefix("{P")
                    .and_then(|value| value.strip_suffix('}'))
                    .and_then(|value| value.parse::<usize>().ok());
                if let Some(player_index) = player_index {
                    let command = run_commands
                        .get(player_index.saturating_sub(1))
                        .with_context(|| format!("cmd_play_match references missing {part}"))?;
                    command_parts.push(command.clone());
                } else {
                    command_parts.push(part);
                }
            }
        }
    }

    Ok((command_parts, replay_path))
}

async fn spawn_play_match_command(
    command_parts: Vec<String>,
    replay_path: Option<PathBuf>,
    worker_path: PathBuf,
    input: PlayMatchInput,
) -> anyhow::Result<PlayMatchOutput> {
    let absolute_replay_path = replay_path
        .as_ref()
        .map(|path| crate::replay_artifact::resolve(&worker_path, path))
        .transpose()?;
    if let Some(parent) = absolute_replay_path.as_ref().and_then(|path| path.parent()) {
        fs::create_dir_all(parent)
            .await
            .context("cannot create replay directory")?;
    }

    let result = async {
        let cmd_output = Command::new(&command_parts[0])
            .args(&command_parts[1..])
            .current_dir(&worker_path)
            .output()
            .await
            .context("Error while executing cmd_play_match")?;

        let result = if cmd_output.status.success() {
            let stdout =
                String::from_utf8(cmd_output.stdout).context("stdout is not valid UTF-8")?;
            serde_json::from_str::<CmdPlayMatchStdout>(&stdout)
                .context("play match output should be valid JSON")?
        } else {
            bail!(
                "Error while running match: {}",
                String::from_utf8(cmd_output.stderr).context("stderr is not valid UTF-8")?
            );
        };

        let persisted_replay_path = if let Some(path) = &absolute_replay_path {
            match fs::metadata(path).await {
                Ok(metadata) if metadata.is_file() && metadata.len() > 0 => replay_path.clone(),
                Ok(_) => {
                    tracing::warn!("match command produced an empty replay artifact");
                    None
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tracing::warn!("match command produced no replay artifact");
                    None
                }
                Err(error) => return Err(error).context("cannot inspect replay artifact"),
            }
        } else {
            None
        };

        Ok(PlayMatchOutput {
            seed: input.seed,
            participants: input
                .bots
                .iter()
                .zip_eq(result.ranks)
                .zip_eq(result.errors)
                .map(|((b, r), e)| Participant {
                    bot_id: b.bot_id,
                    rank: r,
                    error: e == 1,
                })
                .collect(),
            attributes: result
                .attributes
                .into_iter()
                .map(|attr| to_match_attribute(&input, attr))
                .chain(if result.scores.is_empty() {
                    vec![].into_iter()
                } else {
                    input
                        .bots
                        .iter()
                        .zip_eq(result.scores)
                        .map(|(b, s)| MatchAttribute {
                            name: "score".to_string(),
                            bot_id: Some(b.bot_id),
                            turn: None,
                            value: MatchAttributeValue::Integer(s as _),
                        })
                        .collect_vec()
                        .into_iter()
                })
                .collect(),
            replay_path: persisted_replay_path,
        })
    }
    .await;

    if result.is_err() {
        if let Some(path) = replay_path {
            if let Err(error) = crate::replay_artifact::remove(&worker_path, &path).await {
                tracing::warn!("cannot clean failed replay artifact: {error:#}");
            }
        }
    }

    result
}

#[derive(Clone)]
pub struct BuildBotInput {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub source_code: SourceCode,
    pub language: Language,
}

#[derive(Debug)]
pub struct BuildBotOutput {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub result: BuildResult,
}

pub struct PlayMatchInput {
    pub bots: Vec<PlayMatchBot>,
    pub seed: i64,
}

#[derive(Clone)]
pub struct PlayMatchBot {
    pub bot_id: BotId,
    pub language: Language,
}

pub struct PlayMatchOutput {
    pub seed: i64,
    pub participants: Vec<Participant>,
    pub attributes: Vec<MatchAttribute>,
    pub replay_path: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct CmdPlayMatchStdout {
    #[serde(default)]
    pub scores: Vec<i32>,
    pub ranks: Vec<u8>,
    pub errors: Vec<u8>,
    #[serde(default)]
    pub attributes: Vec<CmdMatchAttribute>,
}

#[derive(Deserialize, Default)]
pub struct CmdMatchAttribute {
    pub name: String,
    pub player: Option<usize>,
    pub turn: Option<u16>,
    pub value: String,
}

fn to_match_attribute(input: &PlayMatchInput, attr: CmdMatchAttribute) -> MatchAttribute {
    let bot_id = attr.player.map(|p| input.bots[p].bot_id);

    MatchAttribute {
        name: attr.name,
        bot_id,
        turn: attr.turn,
        value: attr.value.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_match_executions_receive_distinct_replay_paths() {
        let config = EmbeddedWorkerConfig {
            threads: 2,
            cmd_play_match: "runner {SEED} {REPLAY_PATH} {P1} {P2}".to_string(),
            cmd_watch_replay: "viewer".to_string(),
            cmd_build: "builder".to_string(),
            cmd_run: "bot {DIR}".to_string(),
        };
        let input = PlayMatchInput {
            bots: vec![
                PlayMatchBot {
                    bot_id: BotId::from(1),
                    language: "rust".to_string().try_into().unwrap(),
                },
                PlayMatchBot {
                    bot_id: BotId::from(2),
                    language: "rust".to_string().try_into().unwrap(),
                },
            ],
            seed: 42,
        };

        let (first_command, first_path) = prepare_play_match_command(&config, &input).unwrap();
        let (second_command, second_path) = prepare_play_match_command(&config, &input).unwrap();
        let first_path = first_path.unwrap();
        let second_path = second_path.unwrap();

        assert_ne!(first_path, second_path);
        assert_eq!(first_command[1], "42");
        assert_eq!(first_command[2], first_path.to_str().unwrap());
        assert_eq!(second_command[2], second_path.to_str().unwrap());
        assert_eq!(first_command[3], "bot bots/1");
        assert_eq!(first_command[4], "bot bots/2");
    }
}
