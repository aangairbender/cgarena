use crate::domain::{BotId, BotName, BotRole, Language, SourceCode};
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Bot {
    pub id: BotId,
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
    pub role: BotRole,
    pub evaluation_plan_revision_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl Bot {
    pub fn new(
        name: BotName,
        source_code: SourceCode,
        language: Language,
        role: BotRole,
        evaluation_plan_revision_id: Option<i64>,
    ) -> Self {
        Self {
            id: BotId::UNINITIALIZED,
            name,
            source_code,
            language,
            role,
            evaluation_plan_revision_id,
            created_at: Utc::now(),
        }
    }
}
