use crate::config::EmbeddedWorkerConfig;
use crate::domain::{BotId, BuildResult, Language, Participant, SourceCode, WorkerName};
use itertools::Itertools;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::{fs, process::Command};
use tokio_util::sync::CancellationToken;
use tracing::warn;

pub struct EmbeddedWorker {
    worker_path: PathBuf,
    pub config: Arc<EmbeddedWorkerConfig>,
    pub match_tx: Sender<PlayMatchInput>,
    pub match_result_rx: Receiver<PlayMatchOutput>,
}

const DIR_BOTS: &str = "bots";

impl EmbeddedWorker {
    pub fn new(worker_path: &Path, config: EmbeddedWorkerConfig, token: CancellationToken) -> Self {
        let config = Arc::new(config);

        let (match_result_tx, match_result_rx) = channel(100);
        let (match_tx, match_rx) = channel(config.threads as usize * 2);
        tokio::spawn(run_play_matches(
            match_rx,
            worker_path.to_path_buf(),
            Arc::clone(&config),
            match_result_tx,
            token.clone(),
        ));

        Self {
            worker_path: worker_path.to_path_buf(),
            config,
            match_tx,
            match_result_rx,
        }
    }

    pub async fn build_bot(&self, input: BuildBotInput) -> BuildBotOutput {
        let bot_id = input.bot_id.clone();
        let worker_name = input.worker_name.clone();

        let result = build_bot(self.worker_path.clone(), Arc::clone(&self.config), input).await;

        BuildBotOutput {
            bot_id,
            worker_name,
            result,
        }
    }

    pub async fn is_build_valid(&self, id: BotId) -> bool {
        let bot_folder = self
            .worker_path
            .join(DIR_BOTS)
            .join(i64::from(id).to_string());
        tokio::fs::try_exists(&bot_folder).await.unwrap_or(false)
    }
}

async fn build_bot(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    input: BuildBotInput,
) -> BuildResult {
    let bot_folder_relative = PathBuf::from(DIR_BOTS).join(i64::from(input.bot_id).to_string());
    let bot_folder = worker_path.join(&bot_folder_relative);
    if bot_folder.exists() {
        warn!("bot folder already exists, skipping build");
        return BuildResult::Success;
    }

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
}

#[derive(Deserialize)]
pub struct CmdPlayMatchStdout {
    pub ranks: Vec<u8>,
    pub errors: Vec<u8>,
}
