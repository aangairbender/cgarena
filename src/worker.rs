use std::{path::{Path, PathBuf}, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use anyhow::bail;
use tokio::{fs, process::Command, sync::{mpsc, oneshot, Semaphore}};

use crate::{config::WorkerConfig, model::Bot};

struct MatchResult {
    seed: i32,
    bot_ids: Vec<i32>,
    ranks: Vec<usize>,
    errors: Vec<bool>,
}

enum Message {
    BuildBot {
        bot: Bot,
        respond_to: oneshot::Sender<Result<(), anyhow::Error>>,
    },
    EnqueueMatch {
        seed: i32,
        bot_ids: Vec<i32>,
    },
    GetQueueSize {
        respond_to: oneshot::Sender<usize>,
    },
}

struct Actor {
    receiver: mpsc::Receiver<Message>,
    queue_size: Arc<AtomicUsize>,
    dir_languages: PathBuf,
    dir_bots: PathBuf,
    config: WorkerConfig,
    match_result_sender: mpsc::Sender<Result<MatchResult, anyhow::Error>>,
    match_semaphore: Arc<Semaphore>,
}

impl Actor {
    pub async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::BuildBot { bot, respond_to } => {
                let input = BuildBotInput {
                    bot_id: bot.id,
                    source_code: bot.source_code,
                    language: bot.language,
                    dir_bots: self.dir_bots.clone(),
                    dir_languages: self.dir_languages.clone(),
                };
                tokio::spawn(async move {
                    let res = build_bot(input).await;
                    let _ = respond_to.send(res);
                });
            },
            Message::EnqueueMatch { seed, bot_ids } => {
                let queue_size = self.queue_size.clone();
                let match_semaphore = self.match_semaphore.clone();
                let res_sender = self.match_result_sender.clone();
                let input = RunMatchInput {
                    seed,
                    bot_ids,
                    dir_bots: self.dir_bots.clone(),
                    cmd_play_match: self.config.cmd_play_match.clone(),
                };
                tokio::spawn(async move {
                    queue_size.fetch_add(1, Ordering::SeqCst);
                    let _permit = match_semaphore.acquire().await
                        .expect("should be able to aquire semaphore permit");
                    queue_size.fetch_sub(1, Ordering::SeqCst);
                    let res = run_match(input).await;
                    let _ = res_sender.send(res).await;
                });
            },
            Message::GetQueueSize { respond_to } => {
                let sz = self.queue_size.load(Ordering::SeqCst);
                let _ = respond_to.send(sz);
            },
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}


struct RunMatchInput {
    seed: i32,
    bot_ids: Vec<i32>,
    dir_bots: PathBuf,
    cmd_play_match: String,
}

async fn run_match(input: RunMatchInput) -> Result<MatchResult, anyhow::Error> {
    let run_cmds: Vec<String> = input.bot_ids.iter()
        .map(|id| {
            let path = input.dir_bots.join(id.to_string()).join("run.sh");
            let abs_path = std::fs::canonicalize(path).expect("cant get absolute path");
            abs_path.to_str().expect("cant convert path to string").to_owned()
        })
        .collect();

    let output = Command::new("sh")
        .arg("run_match.sh")
        .args(&run_cmds)
        .output()
        .await?;

    if output.status.success() {
        let w = std::str::from_utf8(&output.stdout)?;
        unimplemented!()
    } else {
        bail!(std::str::from_utf8(&output.stderr)?.to_owned())
    }
}

struct BuildBotInput {
    bot_id: i32,
    source_code: String,
    language: String,
    dir_bots: PathBuf,
    dir_languages: PathBuf,
}

struct BuildBotOutput {

}

async fn build_bot(input: BuildBotInput) -> Result<(), anyhow::Error> {
    let lang_folder = input.dir_languages.join(&input.language);
    if !lang_folder.exists() {
        bail!("unsupported language")
    }

    let bot_folder = input.dir_bots.join(input.bot_id.to_string());
    if bot_folder.exists() {
        bail!("bot folder already exists")
    }

    let bot_folder_clone = bot_folder.clone();
    tokio::task::spawn_blocking(move || copy_dir_all(lang_folder, bot_folder_clone)).await??;

    fs::write(bot_folder.join("source.txt"), &input.source_code).await?;

    let output = Command::new("sh")
        .arg("build.sh")
        .current_dir(&bot_folder)
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        bail!(std::str::from_utf8(&output.stderr)?.to_owned())
    }
}