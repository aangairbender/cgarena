use crate::db::{DBError, Database};
use crate::domain::BotId;

pub enum Output {
    Deleted,
    NotFound,
}

pub async fn execute(id: BotId, db: Database) -> Output {
    match db.delete_bot(id).await {
        Ok(()) => Output::Deleted,
        Err(DBError::NotFound) => Output::NotFound,
        _ => panic!("Unexpected error from repo"),
    }
}
