#[derive(Clone)]
pub enum BuildStatus {
    Pending,
    Running,
    Finished(BuildResult),
}

#[derive(Debug, Clone)]
pub enum BuildResult {
    Success,
    Failure { stderr: String },
}
