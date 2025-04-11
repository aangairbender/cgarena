use crate::domain::{Build, BuildResult, BuildStatus};
use serde::Serialize;

#[derive(Serialize)]
pub struct BuildResponse {
    pub worker_name: String,
    pub status: String,
    pub stderr: Option<String>,
}

impl From<Build> for BuildResponse {
    fn from(b: Build) -> Self {
        let (status, stderr) = match b.status {
            BuildStatus::Pending => ("pending".to_string(), None),
            BuildStatus::Running => ("running".to_string(), None),
            BuildStatus::Finished(BuildResult::Success) => ("finished".to_string(), None),
            BuildStatus::Finished(BuildResult::Failure { stderr }) => {
                ("finished".to_string(), Some(stderr))
            }
        };
        BuildResponse {
            worker_name: b.worker_name.into(),
            status,
            stderr,
        }
    }
}
