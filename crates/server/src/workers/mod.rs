mod embedded;
mod worker_thread;

pub use embedded::EmbeddedWorker;
pub use worker_thread::WorkerThread;

use async_trait::async_trait;
use std::collections::HashMap;

use uuid::Uuid;

pub struct Job {
    match_id: Uuid,
    seed: i32,
    bot_ids: Vec<Uuid>,
}
pub struct JobResult {
    match_id: Uuid,
    scores: HashMap<Uuid, i32>,
}

#[async_trait]
pub trait Worker {
    fn name(&self) -> &str;
    async fn queue(&mut self, job: Job);
    async fn fetch_results(&mut self) -> Vec<JobResult>;
}
