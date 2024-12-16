use crate::config::WorkerConfig;
use crate::domain::{BotId, Language, SourceCode, WorkerName};
use crate::embedded_worker::EmbeddedWorker;
use std::path::Path;

pub struct Worker {
    pub name: WorkerName,
    pub threads: u8,
    pub stats: WorkerStats,
    pub inner: EmbeddedWorker,
}

impl Worker {
    pub fn new(arena_path: &Path, config: WorkerConfig) -> Self {
        match config {
            WorkerConfig::Embedded(c) => Worker {
                name: "embedded".to_string().try_into().unwrap(),
                threads: c.threads,
                stats: WorkerStats::default(),
                inner: EmbeddedWorker::new(arena_path, c),
            },
        }
    }

    pub async fn build_bot(&self, input: BuildBotInput) -> Result<(), anyhow::Error> {
        self.inner.build(input).await
    }

    pub async fn is_build_valid(&self, id: BotId) -> bool {
        self.inner.is_build_valid(id).await
    }
}

#[derive(Clone)]
pub struct BuildBotInput {
    pub bot_id: BotId,
    pub source_code: SourceCode,
    pub language: Language,
}

#[derive(Default)]
pub struct WorkerStats {
    pub queue_size: usize,
}
