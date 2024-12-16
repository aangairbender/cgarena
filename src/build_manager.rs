use crate::db::Database;
use crate::domain::{BotId, Build};
use crate::worker::{BuildBotInput, Worker};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::warn;

#[derive(Clone)]
pub struct BuildManager {
    workers: Arc<[Worker]>,
    db: Database,
    pending_builds_tx: Sender<Build>,
}

impl BuildManager {
    pub fn new(workers: Arc<[Worker]>, db: Database) -> Self {
        let (tx, rx) = mpsc::channel(16);

        tokio::spawn(Self::process_pending_builds(
            rx,
            db.clone(),
            Arc::clone(&workers),
        ));

        let res = Self {
            workers,
            db,
            pending_builds_tx: tx,
        };

        tokio::spawn(res.clone().process_existing_bots());
        res
    }

    async fn process_existing_bots(self) {
        let bots = self.db.fetch_bots().await;
        for bot in &bots {
            self.ensure_built(bot.id).await;
        }
    }

    async fn process_pending_builds(mut rx: Receiver<Build>, db: Database, workers: Arc<[Worker]>) {
        while let Some(build) = rx.recv().await {
            let Some(bot) = db.fetch_bot(build.bot_id).await else {
                warn!(
                    "Build: bot {:?} is not present in db, skipping.",
                    build.bot_id
                );
                continue;
            };

            let Some(worker) = workers.iter().find(|w| w.name == build.worker_name) else {
                warn!(
                    "Build: worker {:?} is not present in config, skipping.",
                    build.worker_name
                );
                continue;
            };

            let build = build.into_running();
            db.upsert_build(&build).await;

            let input = BuildBotInput {
                bot_id: build.bot_id,
                source_code: bot.source_code,
                language: bot.language,
            };
            let res = worker.build_bot(input).await;
            let build = build.into_finished(res);
            db.upsert_build(&build).await;
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
        let existing_builds = self.db.fetch_bot_builds(id).await;

        for w in self.workers.as_ref() {
            let build_exists_in_db = existing_builds
                .iter()
                .find(|b| b.worker_name == w.name)
                .map(|b| b.status.is_success())
                .unwrap_or(false);

            let still_valid = w.is_build_valid(id).await;

            if build_exists_in_db && still_valid {
                continue;
            }

            let build = Build::new(id, w.name.clone());
            self.db.upsert_build(&build).await;
            self.pending_builds_tx
                .send(build)
                .await
                .expect("sending pending build to channel failed");
        }
    }
}

// const RUN_QUEUE_SIZE_PER_THREAD: usize = 5;
