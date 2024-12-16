pub enum BuildStatus {
    Pending,
    Running,
    Success,
    Failure(String),
}

impl BuildStatus {
    pub(crate) fn is_success(&self) -> bool {
        match self {
            BuildStatus::Success => true,
            _ => false,
        }
    }
}
