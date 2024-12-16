use crate::domain::{BotId, BotName, Language, Rating, SourceCode};
use chrono::{DateTime, Utc};

pub struct Bot {
    pub id: BotId,
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
    pub created_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct BotStats {
    pub matches_played: i64,
    pub rating: Rating,
    pub matches_with_error: i64,
}

impl Bot {
    pub fn new(name: BotName, source_code: SourceCode, language: Language) -> Self {
        Self {
            id: BotId::UNINITIALIZED,
            name,
            source_code,
            language,
            created_at: Utc::now(),
        }
    }
}
