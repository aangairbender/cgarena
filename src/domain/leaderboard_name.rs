use anyhow::bail;
use std::ops::Deref;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct LeaderboardName(String);

impl LeaderboardName {
    pub fn global() -> LeaderboardName {
        LeaderboardName("Global".to_string())
    }
}

impl TryFrom<String> for LeaderboardName {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.is_empty() {
            bail!("LeaderboardName cannot be empty");
        }
        if src.len() >= LEN_LIMIT {
            bail!(
                "LeaderboardName should be less than {} characters",
                LEN_LIMIT
            );
        }
        Ok(Self(src))
    }
}

impl From<LeaderboardName> for String {
    fn from(value: LeaderboardName) -> Self {
        value.0
    }
}

impl Deref for LeaderboardName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

const LEN_LIMIT: usize = 64;
