mod worker;
mod worker_thread;

pub use worker::Worker;
use worker_thread::WorkerThread;

#[derive(Clone)]
pub struct Job {
    pub r#match: r#match::Model,
    pub bots: Vec<bot::Model>,
}
pub struct JobResult {
    pub scores: [i32; 8],
}
