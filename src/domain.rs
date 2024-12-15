use chrono::{DateTime, Utc};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct BotId(pub i32);

pub struct Bot {
    pub id: BotId,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub enum DomainEvent {
    BotCreated(BotId),
}
