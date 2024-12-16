use crate::config::EmbeddedWorkerConfig;
use crate::domain::BotId;
use crate::worker::{BuildBotInput, CmdPlayMatchStdout, PlayMatchInput, PlayMatchOutput};
use anyhow::bail;
use itertools::Itertools;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{fs, process::Command};

pub struct EmbeddedWorker {
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    match_tx: Sender<PlayMatchInput>,
}

const DIR_BOTS: &str = "bots";

impl EmbeddedWorker {
    pub fn new(
        worker_path: &Path,
        config: EmbeddedWorkerConfig,
        match_result_tx: Sender<PlayMatchOutput>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(config.threads as usize * 10);
        let config = Arc::new(config);
        tokio::spawn(Self::play_matches(
            rx,
            worker_path.to_path_buf(),
            Arc::clone(&config),
            match_result_tx,
        ));

        Self {
            worker_path: worker_path.to_path_buf(),
            config,
            match_tx: tx,
        }
    }

    async fn play_matches(
        mut rx: Receiver<PlayMatchInput>,
        worker_path: PathBuf,
        config: Arc<EmbeddedWorkerConfig>,
        match_result_tx: Sender<PlayMatchOutput>,
    ) {
        while let Some(input) = rx.recv().await {
            let run_commands = input
                .bots
                .iter()
                .map(|b| {
                    let bot_folder_relative =
                        PathBuf::from(DIR_BOTS).join(i64::from(b.bot_id).to_string());
                    let dir_param_value = bot_folder_relative.to_str().unwrap();
                    let command = config
                        .cmd_run
                        .replace("{DIR}", dir_param_value)
                        .replace("{LANG}", &b.language);
                    command
                })
                .collect_vec();

            let run_commands_combined = run_commands.join(" ");

            let command_parts = config
                .cmd_play_match
                .split_ascii_whitespace()
                .map(|s| match s {
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
                .collect_vec();
            assert_ne!(command_parts.len(), 0);

            let cmd_output = Command::new(command_parts[0])
                .args(&command_parts[1..])
                .current_dir(&worker_path)
                .output()
                .await
                .expect("Cannot run match");

            let result = if cmd_output.status.success() {
                let stdout =
                    String::from_utf8(cmd_output.stdout).expect("stdout is not valid UTF-8");
                let match_result: CmdPlayMatchStdout =
                    serde_json::from_str(&stdout).expect("play match output should be valid JSON");
                match_result
            } else {
                panic!(
                    "{}",
                    String::from_utf8(cmd_output.stderr).expect("stderr is not valid UTF-8")
                )
            };

            let output = PlayMatchOutput {
                seed: input.seed,
                bot_ids: input.bots.into_iter().map(|b| b.bot_id).collect(),
                result,
            };

            match_result_tx
                .send(output)
                .await
                .expect("Cannot send match result");
        }
    }

    pub async fn enqueue_match(&self, input: PlayMatchInput) {
        self.match_tx
            .send(input)
            .await
            .expect("Failed to send input to match channel");
    }

    pub async fn is_build_valid(&self, id: BotId) -> bool {
        let bot_folder = self
            .worker_path
            .join(DIR_BOTS)
            .join(i64::from(id).to_string());
        tokio::fs::try_exists(&bot_folder).await.unwrap_or(false)
    }

    pub async fn build(&self, input: BuildBotInput) -> Result<(), anyhow::Error> {
        let bot_folder_relative = PathBuf::from(DIR_BOTS).join(i64::from(input.bot_id).to_string());
        let bot_folder = self.worker_path.join(&bot_folder_relative);
        if bot_folder.exists() {
            bail!("bot folder already exists")
        }

        fs::create_dir_all(&bot_folder).await?;
        fs::write(
            bot_folder.join("source.txt"),
            &String::from(input.source_code),
        )
        .await?;

        let dir_param_value = bot_folder_relative.to_str().unwrap();
        let command_parts = self
            .config
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
            .current_dir(&self.worker_path)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(
                std::str::from_utf8(&output.stderr)?.to_owned(),
            ))
        }
    }
}
