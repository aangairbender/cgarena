use crate::config::WorkerConfig;
use crate::db::Database;
use crate::domain::{BotId, DomainEvent};
use crate::embedded_worker::EmbeddedWorker;
use crate::worker::{BuildInput, BuildOutput, Worker};
use itertools::Itertools;
use std::path::Path;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;

pub struct WorkerManager {
    workers: Vec<WorkerData>,
    db: Database,
    receiver: Receiver<DomainEvent>,
}

impl WorkerManager {
    pub fn new(arena_path: &Path, worker_configs: Vec<WorkerConfig>, db: Database) -> Self {
        let workers: Vec<WorkerData> = worker_configs
            .into_iter()
            .map(|config| match config {
                WorkerConfig::Embedded(c) => WorkerData {
                    handle: Worker::Embedded(EmbeddedWorker::new(arena_path, c)),
                },
            })
            .collect();
        assert!(
            workers.iter().map(|w| w.handle.name()).all_unique(),
            "All worker names must be unique"
        );
        Self {
            workers,
            receiver: db.subscribe(),
            db,
        }
    }

    pub async fn run(mut self, token: CancellationToken) {
        loop {
            select! {
                _ = token.cancelled() => break,
                event = self.receiver.recv() => {
                    let Ok(event) = event else { break };
                    self.handle_event(event).await;
                }
            }
        }
    }

    async fn handle_event(&self, event: DomainEvent) {
        match event {
            DomainEvent::BotCreated(id) => {
                self.build_bot(id).await;
            }
        }
    }

    async fn build_bot(&self, id: BotId) {
        let existing_builds = self.db.fetch_builds(id).await;

        let target_workers = self
            .workers
            .iter()
            .filter(|w| {
                !existing_builds
                    .iter()
                    .any(|b| b.worker_name == w.handle.name())
            })
            .collect_vec();

        if target_workers.is_empty() {
            return;
        }

        let Some(bot) = self.db.fetch_bot(id).await else {
            return;
        };
        let input = BuildInput {
            bot_id: id,
            source_code: bot.source_code,
            language: bot.language,
        };
        for w in target_workers {
            let bot_id = input.bot_id;
            let res = w
                .handle
                .build(input.clone())
                .await
                .unwrap_or_else(|err| BuildOutput {
                    status_code: None,
                    stdout: None,
                    stderr: Some(err.to_string()),
                });
            self.db.insert_build(bot_id, w.handle.name(), res).await;
        }
    }
}

struct WorkerData {
    handle: Worker,
    // some stats
}
