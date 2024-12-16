use crate::domain::{BotId, BuildStatus, WorkerName};

pub struct Build {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub status: BuildStatus,
}

impl Build {
    pub fn new(bot_id: BotId, worker_name: WorkerName) -> Self {
        Self {
            bot_id,
            worker_name,
            status: BuildStatus::Pending,
        }
    }

    pub fn reset(mut self) -> Self {
        self.status = BuildStatus::Pending;
        self
    }

    pub fn into_running(mut self) -> Self {
        assert_eq!(
            std::mem::discriminant(&self.status),
            std::mem::discriminant(&BuildStatus::Pending)
        );
        self.status = BuildStatus::Running;
        self
    }

    pub fn into_finished(mut self, res: Result<(), anyhow::Error>) -> Self {
        assert_eq!(
            std::mem::discriminant(&self.status),
            std::mem::discriminant(&BuildStatus::Running)
        );
        self.status = match res {
            Ok(()) => BuildStatus::Success,
            Err(err) => BuildStatus::Failure(err.to_string()),
        };
        self
    }
}
