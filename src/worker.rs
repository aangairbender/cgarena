use std::{collections::VecDeque, path::{Path, PathBuf}};

use anyhow::bail;
use tokio::{fs, process::Command, sync::{mpsc, oneshot}};

use crate::{config::WorkerConfig, model::Bot};

struct Match {
    seed: i32,
    bot_ids: Vec<i32>,
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
    queue: VecDeque<Match>,
    dir_languages: PathBuf,
    dir_bots: PathBuf,
    config: WorkerConfig,
}

impl Actor {
    pub async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::BuildBot { bot, respond_to } => {
                let res = self.build_bot(&bot).await;
                let _ = respond_to.send(res);
            },
            Message::EnqueueMatch { seed, bot_ids } => {
                self.queue.push_back(Match { seed, bot_ids })
            },
            Message::GetQueueSize { respond_to } => {
                let _ = respond_to.send(self.queue.len());
            },
        }
    }

    async fn build_bot(&mut self, bot: &Bot) -> Result<(), anyhow::Error> {
        let lang_folder = self.dir_languages.join(&bot.language);
        if !lang_folder.exists() {
            bail!("unsupported language")
        }

        let bot_folder = self.dir_bots.join(bot.id.to_string());
        if bot_folder.exists() {
            bail!("bot folder already exists")
        }

        let bot_folder_clone = bot_folder.clone();
        tokio::task::spawn_blocking(move || copy_dir_all(lang_folder, bot_folder_clone)).await??;

        fs::write(bot_folder.join("source.txt"), &bot.source_code).await?;

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

pub struct WorkerCluster {
    receiver: mpsc::Receiver<i32>
}