use crate::domain::BotRole;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateBotRequest {
    pub name: String,
    pub source_code: String,
    pub language: String,
    #[serde(default)]
    pub role: BotRole,
}
