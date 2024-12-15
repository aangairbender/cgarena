use anyhow::bail;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct WorkerName(pub String);

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

const LEN_LIMIT: usize = 32;
