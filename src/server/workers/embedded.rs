use async_trait::async_trait;

use super::{Worker, Job, JobResult, WorkerThread};


pub struct EmbeddedWorker {
    worker_threads: Vec<WorkerThread>,
    jobs_queued: usize,
}

impl EmbeddedWorker {
    pub fn new(threads: u8) -> Self {
        assert!(threads > 0, "Can't start worker with 0 threads");
        let worker_threads = (0..threads).map(|_| {
            WorkerThread::spawn()
        }).collect();
        log::info!("Embedded worker with {} worker threads created", threads);
        Self { worker_threads, jobs_queued: 0 }
    }
}

#[async_trait]
impl Worker for EmbeddedWorker {
    fn name(&self) -> &str { "embedded" }
    
    async fn queue(&mut self, job: Job) {
        let index = self.jobs_queued % self.worker_threads.len();
        self.worker_threads[index].queue(job);
        self.jobs_queued += 1;
    }

    async fn fetch_results(&mut self) -> Vec<JobResult> {
        self.worker_threads.iter_mut()
            .flat_map(WorkerThread::fetch_results)
            .collect()
    }
}
