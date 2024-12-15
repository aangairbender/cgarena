use anyhow::bail;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct BotName(String);

impl TryFrom<String> for BotName {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        if src.is_empty() {
            bail!("Bot name cannot be empty");
        }
        if src.len() >= LEN_LIMIT {
            bail!("BotName should be less than {} characters", LEN_LIMIT);
        }
        Ok(Self(src))
    }
}

impl From<BotName> for String {
    fn from(value: BotName) -> Self {
        value.0
    }
}

const LEN_LIMIT: usize = 32;
