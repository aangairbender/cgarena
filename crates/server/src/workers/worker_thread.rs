use std::thread;
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};

use super::{Job, JobResult};

pub struct WorkerThread {
    job_tx: UnboundedSender<Job>,
    res_rx: UnboundedReceiver<JobResult>,
}

impl WorkerThread {
    pub fn spawn() -> Self {
        let (job_tx, job_rx) = mpsc::unbounded_channel::<Job>();
        let (res_tx, res_rx) = mpsc::unbounded_channel::<JobResult>();
        spawn_worker_thread(job_rx, res_tx);
        WorkerThread { job_tx, res_rx }
    }

    // TODO: deal with infinite recursion
    pub fn queue(&mut self, job: Job) {
        if let Err(e) = self.job_tx.send(job) {
            log::debug!("Cannot send job to the worker thread, respawning the worker thread");
            self.respawn();
            self.queue(e.0);
        }
    }

    // TODO: handle the case when due to restart we are losing some job results, need to redo those
    pub fn fetch_results(&mut self) -> Vec<JobResult> {
        let mut res = Vec::new();
        loop {
            match self.res_rx.try_recv() {
                Ok(job_result) => res.push(job_result),
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    log::debug!(
                        "Cannot fetch results from the worker thread, respawning the worker thread"
                    );
                    self.respawn();
                    break;
                }
            }
        }
        res
    }

    // the old thread should automatically close because we drop the old channels
    fn respawn(&mut self) {
        *self = Self::spawn();
    }
}

fn spawn_worker_thread(mut receiver: UnboundedReceiver<Job>, sender: UnboundedSender<JobResult>) {
    thread::spawn(move || loop {
        match receiver.try_recv() {
            Ok(job) => {
                let res = process_job(job);
                if let Err(_) = sender.send(res) {
                    log::debug!("JobResult receiver disconnected, terminating the worker thread");
                    break;
                }
            }
            Err(TryRecvError::Empty) => {
                continue;
            }
            Err(TryRecvError::Disconnected) => {
                log::debug!("Job sender disconnected, terminating the worker thread");
                break;
            }
        }
    });
}

fn process_job(job: Job) -> JobResult {
    unimplemented!()
}
