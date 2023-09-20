use uuid::Uuid;
use serde::{Deserialize, Serialize};

use super::Language;

#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub source_filename: String,
    pub language: Language,
    pub status: BotStatus,
    pub build_output: String,
}

impl Bot {
    pub fn new(name: String, source_filename: String, language: Language) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            source_filename,
            language,
            status: BotStatus::Pending,
            build_output: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum BotStatus {
    Pending,
    Building,
    Ready,
}
