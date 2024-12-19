use crate::domain::{BotId, BuildResult, BuildStatus, WorkerName};

#[derive(Clone)]
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

    pub fn reset(&mut self) {
        self.status = BuildStatus::Pending;
    }

    pub fn make_running(&mut self) {
        assert_eq!(
            std::mem::discriminant(&self.status),
            std::mem::discriminant(&BuildStatus::Pending)
        );
        self.status = BuildStatus::Running;
    }

    pub fn make_finished(&mut self, result: BuildResult) {
        assert_eq!(
            std::mem::discriminant(&self.status),
            std::mem::discriminant(&BuildStatus::Running)
        );
        self.status = BuildStatus::Finished(result);
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, BuildStatus::Pending)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, BuildStatus::Running)
    }

    pub fn was_finished_successfully(&self) -> bool {
        matches!(self.status, BuildStatus::Finished(BuildResult::Success))
    }
}
