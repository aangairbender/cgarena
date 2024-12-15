use crate::config::WorkerConfig;
use crate::db::Database;
use crate::domain::{BotId, Build, BuildStatus};
use crate::worker::{BuildBotInput, Worker};
use itertools::Itertools;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkerManager {
    workers: Arc<Vec<Worker>>,
    db: Database,
}

impl WorkerManager {
    pub fn new(arena_path: &Path, worker_configs: Vec<WorkerConfig>, db: Database) -> Self {
        let workers = worker_configs
            .into_iter()
            .map(|config| Worker::new(arena_path, config))
            .collect_vec();

        assert!(
            workers.iter().map(|w| &w.name).all_unique(),
            "All worker names must be unique"
        );

        Self {
            workers: Arc::new(workers),
            db,
        }
    }

    // async fn optimal_parallelism(workers: &[WorkerData]) -> usize {
    //     let total_threads = {
    //         let mut res: usize = 0;
    //         for w in workers {
    //             res += w.handle.stats().await.threads as usize;
    //         }
    //         res
    //     };
    //     RUN_QUEUE_SIZE_PER_THREAD * total_threads
    // }

    pub async fn ensure_built(&self, id: BotId) {
        let existing_builds = self.db.fetch_builds(id).await;

        let target_workers = {
            let mut res = Vec::with_capacity(self.workers.len());
            for w in self.workers.as_ref() {
                let build_exists_in_db = existing_builds.iter().any(|b| b.worker_name == w.name);
                let still_valid = w.is_build_valid(id).await;
                let should_build = !(build_exists_in_db && still_valid);
                if should_build {
                    res.push(w);
                }
            }
            res
        };

        if target_workers.is_empty() {
            return;
        }

        let Some(bot) = self.db.fetch_bot(id).await else {
            return;
        };
        let input = BuildBotInput {
            bot_id: id,
            source_code: bot.source_code,
            language: bot.language,
        };
        for w in target_workers {
            let bot_id = input.bot_id;
            let res = w.build(input.clone()).await;
            let build = Build {
                bot_id,
                worker_name: w.name.clone(),
                status: match res {
                    Ok(()) => BuildStatus::Success,
                    Err(err) => BuildStatus::Failure(err.to_string()),
                },
            };
            self.db.insert_build(build).await;
        }
    }
}

// const RUN_QUEUE_SIZE_PER_THREAD: usize = 5;
