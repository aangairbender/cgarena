use config::WorkerConfig;
use tracing::info;

use super::{Job, JobResult, WorkerThread};

pub struct Worker {
    worker_threads: Vec<WorkerThread>,
    jobs_queued_total: usize,
}

impl Worker {
    pub fn new(config: WorkerConfig) -> Self {
        assert!(config.threads > 0, "Can't start worker with 0 threads");
        let worker_threads = (0..config.threads)
            .map(|_| WorkerThread::spawn(config.clone()))
            .collect();
        info!(
            "Embedded worker with {} worker threads created",
            config.threads
        );
        Self {
            worker_threads,
            jobs_queued_total: 0,
        }
    }
}

impl Worker {
    pub async fn run(&mut self, job: Job) -> Result<JobResult, anyhow::Error> {
        let index = self.jobs_queued_total % self.worker_threads.len();
        self.jobs_queued_total += 1;
        // TODO: implement retry logic
        self.worker_threads[index].run(job).await
    }
}
