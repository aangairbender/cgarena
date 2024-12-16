use crate::db::Database;

use crate::domain::BotId;
use crate::worker::{PlayMatchBot, PlayMatchInput, Worker};
use itertools::Itertools;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::warn;

pub async fn run(db: Database, workers: Arc<[Worker]>, mut rx: Receiver<ScheduledMatch>) {
    let mut worker_index: usize = 0;
    while let Some(scheduled) = rx.recv().await {
        worker_index = (worker_index + 1) % workers.len();

        let mut bots = Vec::with_capacity(scheduled.bot_ids.len());
        for bot_id in scheduled.bot_ids {
            let Some(bot) = db.fetch_bot(bot_id).await else {
                warn!("Cant schedule a match with non-existent bot");
                return;
            };
            bots.push(PlayMatchBot {
                bot_id: bot.id,
                language: bot.language,
            });
        }
        let input = PlayMatchInput {
            bots,
            seed: scheduled.seed,
        };

        workers[worker_index].enqueue_match(input).await;
    }
}

#[derive(Clone)]
pub struct MatchScheduler {
    tx: Sender<ScheduledMatch>,
}

impl MatchScheduler {
    pub fn new(tx: Sender<ScheduledMatch>) -> Self {
        Self { tx }
    }

    pub async fn schedule(&self, scheduled_match: ScheduledMatch) {
        self.tx
            .send(scheduled_match)
            .await
            .expect("Cannot schedule match");
    }
}

pub struct ScheduledMatch {
    pub seed: i64,
    pub bot_ids: Vec<BotId>,
}

impl ScheduledMatch {
    pub fn into_permutations(self) -> Vec<Self> {
        let n = self.bot_ids.len();
        self.bot_ids
            .into_iter()
            .permutations(n)
            .map(|p| ScheduledMatch {
                seed: self.seed,
                bot_ids: p,
            })
            .collect()
    }
}
