use crate::arena::BotMinimal;
use serde::Serialize;

#[derive(Serialize)]
pub struct BotMinimalResponse {
    pub id: i64,
    pub name: String,
}

impl From<BotMinimal> for BotMinimalResponse {
    fn from(value: BotMinimal) -> Self {
        BotMinimalResponse {
            id: value.id.into(),
            name: value.name.into(),
        }
    }
}
