use crate::config::WorkerConfig;
use crate::domain::{BotId, Language, SourceCode, WorkerName};
use crate::embedded_worker::EmbeddedWorker;
use serde::Deserialize;
use std::path::Path;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

pub struct Worker {
    pub name: WorkerName,
    pub threads: u8,
    pub inner: EmbeddedWorker,
}

impl Worker {
    pub fn new(
        arena_path: &Path,
        config: WorkerConfig,
        match_result_tx: Sender<PlayMatchOutput>,
        token: CancellationToken,
    ) -> Self {
        match config {
            WorkerConfig::Embedded(c) => Worker {
                name: "embedded".to_string().try_into().unwrap(),
                threads: c.threads,
                inner: EmbeddedWorker::new(arena_path, c, match_result_tx, token),
            },
        }
    }

    pub async fn build_bot(&self, input: BuildBotInput) -> Result<(), anyhow::Error> {
        self.inner.build(input).await
    }

    pub async fn is_build_valid(&self, id: BotId) -> bool {
        self.inner.is_build_valid(id).await
    }

    pub async fn enqueue_match(&self, input: PlayMatchInput) {
        self.inner.enqueue_match(input).await
    }
}

#[derive(Clone)]
pub struct BuildBotInput {
    pub bot_id: BotId,
    pub source_code: SourceCode,
    pub language: Language,
}

pub struct PlayMatchInput {
    pub bots: Vec<PlayMatchBot>,
    pub seed: i64,
}

pub struct PlayMatchBot {
    pub bot_id: BotId,
    pub language: Language,
}

pub struct PlayMatchOutput {
    pub seed: i64,
    pub bot_ids: Vec<BotId>,
    pub result: CmdPlayMatchStdout,
}

#[derive(Deserialize)]
pub struct CmdPlayMatchStdout {
    pub ranks: Vec<u8>,
    pub errors: Vec<u8>,
}
