use anyhow::bail;

#[derive(Clone)]
pub struct SourceCode(String);

impl TryFrom<String> for SourceCode {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.len() >= LEN_LIMIT {
            bail!("Source code should be less than {} characters", LEN_LIMIT);
        }
        Ok(SourceCode(src))
    }
}

impl From<SourceCode> for String {
    fn from(value: SourceCode) -> Self {
        value.0
    }
}

const LEN_LIMIT: usize = 100_000;
