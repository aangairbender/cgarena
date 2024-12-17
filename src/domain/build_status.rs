pub enum BuildStatus {
    Pending,
    Running,
    Finished(BuildResult),
}

#[derive(Debug)]
pub enum BuildResult {
    Success,
    Failure { stderr: String },
}
