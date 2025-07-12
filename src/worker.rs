use crate::config::EmbeddedWorkerConfig;
use crate::domain::{
    BotId, BuildResult, Language, MatchAttribute, Participant, SourceCode, WorkerName,
};
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
    token: CancellationToken,
) -> WorkerHandle {
    let config = Arc::new(config);

    let known_bot_ids = known_bot_ids(worker_path);

    let (match_result_tx, match_result_rx) = channel(100);
    let (match_tx, match_rx) = channel(config.threads as usize * 2);
    tokio::spawn(run_play_matches(
        match_rx,
        worker_path.to_path_buf(),
        Arc::clone(&config),
        match_result_tx,
        token.clone(),
    ));

    let (build_tx, build_rx) = channel(1);
    tokio::spawn(run_build_bots(worker_path.to_path_buf(), config, build_rx));

    WorkerHandle {
        match_tx,
        match_result_rx,
        build_tx,
        known_bot_ids,
    }
}

fn known_bot_ids(worker_path: &Path) -> Vec<BotId> {
    let bots_folder = worker_path.join(DIR_BOTS);
    let mut res = vec![];

    if !bots_folder.exists() {
        return vec![];
    }

    for entry in std::fs::read_dir(bots_folder).unwrap() {
        let entry = entry.unwrap();
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

    res
}

async fn run_build_bots(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    mut rx: Receiver<BuildCmd>,
) {
    while let Some(cmd) = rx.recv().await {
        let bot_id = cmd.input.bot_id;
        let worker_name = cmd.input.worker_name.clone();

        let result = build_bot(worker_path.clone(), Arc::clone(&config), cmd.input).await;

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
) -> BuildResult {
    let bot_folder_relative = PathBuf::from(DIR_BOTS).join(i64::from(input.bot_id).to_string());
    let bot_folder = worker_path.join(&bot_folder_relative);

    fs::create_dir_all(&bot_folder)
        .await
        .expect("Failed to create bot folder");
    fs::write(
        bot_folder.join("source.txt"),
        &String::from(input.source_code),
    )
    .await
    .expect("Cannot create source.txt file");

    let dir_param_value = bot_folder_relative.to_str().unwrap();
    let command_parts = config
        .cmd_build
        .split_ascii_whitespace()
        .map(|s| match s {
            "{DIR}" => dir_param_value,
            "{LANG}" => &input.language,
            _ => s,
        })
        .collect_vec();
    assert_ne!(command_parts.len(), 0);

    let output = Command::new(command_parts[0])
        .args(&command_parts[1..])
        .current_dir(&worker_path)
        .output()
        .await
        .expect("Failed to execute command");

    if output.status.success() {
        BuildResult::Success
    } else {
        BuildResult::Failure {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }
}

async fn run_play_matches(
    mut rx: Receiver<PlayMatchInput>,
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    match_result_tx: Sender<PlayMatchOutput>,
    token: CancellationToken,
) {
    let semaphore = Arc::new(Semaphore::new(config.threads as usize));

    while let Some(input) = rx.recv().await {
        if token.is_cancelled() {
            break;
        }

        let semaphore = Arc::clone(&semaphore);
        let permit = semaphore.acquire_owned().await.expect("Semaphore poisoned");
        let run_commands = input
            .bots
            .iter()
            .map(|b| {
                let bot_folder_relative =
                    PathBuf::from(DIR_BOTS).join(i64::from(b.bot_id).to_string());
                let dir_param_value = bot_folder_relative.to_str().unwrap();
                config
                    .cmd_run
                    .replace("{DIR}", dir_param_value)
                    .replace("{LANG}", &b.language)
            })
            .collect_vec();

        let run_commands_combined = run_commands.join(" ");
        let seed = input.seed.to_string();

        let command_parts = config
            .cmd_play_match
            .split_ascii_whitespace()
            .map(|s| match s {
                "{SEED}" => &seed,
                "{P1}" => &run_commands[0],
                "{P2}" => &run_commands[1],
                "{P3}" => &run_commands[2],
                "{P4}" => &run_commands[3],
                "{P5}" => &run_commands[4],
                "{P6}" => &run_commands[5],
                "{P7}" => &run_commands[6],
                "{P8}" => &run_commands[7],
                "{PLAYERS}" => &run_commands_combined,
                _ => s,
            })
            .map(|s| s.to_string())
            .collect_vec();
        assert_ne!(command_parts.len(), 0);

        let match_result_tx_clone = match_result_tx.clone();
        let worker_path_clone = worker_path.clone();
        tokio::spawn(async move {
            spawn_play_match_command(
                command_parts,
                worker_path_clone,
                input,
                match_result_tx_clone,
            )
            .await;
            drop(permit);
        });
    }
}

async fn spawn_play_match_command(
    command_parts: Vec<String>,
    worker_path: PathBuf,
    input: PlayMatchInput,
    match_result_tx: Sender<PlayMatchOutput>,
) {
    let cmd_output = Command::new(&command_parts[0])
        .args(&command_parts[1..])
        .current_dir(&worker_path)
        .output()
        .await
        .expect("Cannot run match");

    let result = if cmd_output.status.success() {
        let stdout = String::from_utf8(cmd_output.stdout).expect("stdout is not valid UTF-8");
        let match_result: CmdPlayMatchStdout =
            serde_json::from_str(&stdout).expect("play match output should be valid JSON");
        match_result
    } else {
        panic!(
            "Error while running match: {}",
            String::from_utf8(cmd_output.stderr).expect("stderr is not valid UTF-8")
        );
    };

    let output = PlayMatchOutput {
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
            .unique_by(|a| (a.name.clone(), a.bot_id, a.turn))
            .collect(),
    };

    match_result_tx
        .send(output)
        .await
        .expect("Cannot send match result");
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
}

#[derive(Deserialize)]
pub struct CmdPlayMatchStdout {
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
