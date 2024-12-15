use crate::db::{DBError, Database};
use crate::domain::{BotId, BotName};

pub struct Input {
    pub bot_id: BotId,
    pub new_name: BotName,
}

pub enum Output {
    Ok,
    NotFound,
    AlreadyExists,
}

pub async fn execute(input: Input, db: Database) -> Output {
    match db.rename_bot(input.bot_id, input.new_name).await {
        Ok(()) => Output::Ok,
        Err(DBError::NotFound) => Output::NotFound,
        Err(DBError::AlreadyExists) => Output::AlreadyExists,
    }
}
