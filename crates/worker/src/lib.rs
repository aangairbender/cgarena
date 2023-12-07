//mod embedded;
//mod worker_thread;

//pub use embedded::EmbeddedWorker;
//pub use worker_thread::WorkerThread;

//use async_trait::async_trait;
use std::collections::HashMap;

pub struct Job {
    match_id: i32,
    seed: i32,
    bot_ids: Vec<i32>,
}
pub struct JobResult {
    match_id: i32,
    scores: HashMap<i32, i32>,
}

//#[async_trait]
pub trait Worker {
    fn name(&self) -> &str;
    fn queue(&mut self, job: Job);
    fn fetch_results(&mut self) -> Vec<JobResult>;
}
