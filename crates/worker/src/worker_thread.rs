use config::WorkerConfig;
use std::{
    collections::{HashMap, HashSet},
    thread,
    time::Duration,
};
use tokio::sync::{
    mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tracing::{debug, warn};

use super::{Job, JobResult};

pub struct WorkerThread {
    config: WorkerConfig,
    job_tx: UnboundedSender<(Job, oneshot::Sender<JobResult>)>,
}

impl WorkerThread {
    pub fn spawn(config: WorkerConfig) -> Self {
        let (job_tx, job_rx) = mpsc::unbounded_channel::<(Job, oneshot::Sender<JobResult>)>();
        spawn_worker_thread(config.clone(), job_rx);
        WorkerThread { config, job_tx }
    }

    pub async fn run(&mut self, job: Job) -> Result<JobResult, anyhow::Error> {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.job_tx.send((job.clone(), tx)) {
            warn!("Cannot send job to the worker thread, respawning the worker thread");
            self.respawn();
            return Err(e.into());
        }

        match rx.await {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!(
                    "Cannot receieve result from the worker thread, respawning the worker thread"
                );
                self.respawn();
                return Err(e.into());
            }
        }
    }

    // the old thread should automatically close because we drop the old channels
    fn respawn(&mut self) {
        *self = Self::spawn(self.config.clone());
    }
}

fn spawn_worker_thread(
    config: WorkerConfig,
    mut receiver: UnboundedReceiver<(Job, oneshot::Sender<JobResult>)>,
) {
    thread::spawn(move || loop {
        match receiver.try_recv() {
            Ok((job, tx)) => {
                let res = process_job(&config, job);
                if let Err(_) = tx.send(res) {
                    debug!("JobResult receiver disconnected, terminating the worker thread");
                    break;
                }
            }
            Err(TryRecvError::Empty) => {
                continue;
            }
            Err(TryRecvError::Disconnected) => {
                debug!("Job sender disconnected, terminating the worker thread");
                break;
            }
        }
    });
}

fn process_job(config: &WorkerConfig, job: Job) -> JobResult {
    thread::sleep(Duration::from_secs(1));
    let mut scores = [0; 8];
    for (i, bot) in job.bots.iter().enumerate() {
        scores[i] = i as i32;
    }
    JobResult { scores }
}
