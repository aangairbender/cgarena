use crate::domain::{BotId, BuildStatus, WorkerName};

pub struct Build {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub status: BuildStatus,
}
