use anyhow::bail;
use std::ops::Deref;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct Language(String);

impl TryFrom<String> for Language {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.is_empty() {
            bail!("Language cannot be empty");
        }
        if src.len() >= LEN_LIMIT {
            bail!("Language should be less than {} characters", LEN_LIMIT);
        }
        Ok(Self(src))
    }
}

impl From<Language> for String {
    fn from(value: Language) -> Self {
        value.0
    }
}

impl Deref for Language {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

const LEN_LIMIT: usize = 32;
