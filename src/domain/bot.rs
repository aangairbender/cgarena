use crate::domain::{BotId, BotName, Language, Rating, SourceCode};
use chrono::{DateTime, Utc};

pub struct Bot {
    pub id: BotId,
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
    pub matches_played: u64,
    pub rating: Rating,
    pub created_at: DateTime<Utc>,
}

impl Bot {
    pub fn new(name: BotName, source_code: SourceCode, language: Language) -> Self {
        Self {
            id: BotId::UNINITIALIZED,
            name,
            source_code,
            language,
            matches_played: 0,
            rating: Rating::default(),
            created_at: Utc::now(),
        }
    }
}
