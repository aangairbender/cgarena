use crate::db::{DBError, Database};
use crate::domain::{Bot, BotId, BotName, Language, SourceCode};
use crate::worker_manager::WorkerManager;

pub struct Input {
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
}

pub enum Output {
    Created(BotId),
    AlreadyExists,
}

pub async fn execute(input: Input, db: Database, wm: WorkerManager) -> Output {
    let bot = Bot::new(input.name, input.source_code, input.language);
    let bot_id = match db.create_bot(bot).await {
        Ok(bot) => bot,
        Err(DBError::AlreadyExists) => return Output::AlreadyExists,
        _ => panic!("Unexpected error from repo"),
    };

    wm.ensure_built(bot_id).await;

    Output::Created(bot_id)
}
