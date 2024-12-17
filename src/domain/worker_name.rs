use anyhow::bail;
use std::ops::Deref;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct WorkerName(String);

impl WorkerName {
    pub fn embedded() -> WorkerName {
        WorkerName("embedded".to_string())
    }
}

impl TryFrom<String> for WorkerName {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.is_empty() {
            bail!("WorkerName cannot be empty");
        }
        if src.len() >= LEN_LIMIT {
            bail!("WorkerName should be less than {} characters", LEN_LIMIT);
        }
        Ok(Self(src))
    }
}

impl From<WorkerName> for String {
    fn from(value: WorkerName) -> Self {
        value.0
    }
}

impl Deref for WorkerName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

const LEN_LIMIT: usize = 32;
