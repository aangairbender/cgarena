use std::{
    fs,
    path::{Path, PathBuf},
};

use sqlx::{Pool, Sqlite};

use crate::models::{Bot, Language};

pub struct BotService {
    bots_dir: PathBuf,
    pool: Pool<Sqlite>,
}

#[derive(thiserror::Error, Debug)]
pub enum AddBotError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] sqlx::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum RemoveBotError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] sqlx::Error),
}

impl BotService {
    pub fn new(bots_dir: &Path, pool: Pool<Sqlite>) -> Self {
        Self {
            bots_dir: bots_dir.to_owned(),
            pool,
        }
    }

    pub async fn add_bot(
        &self,
        name: String,
        source_code: String,
        language: Language,
    ) -> Result<(), AddBotError> {
        let source_filename = format!("{}.{}", name, language.file_extension());
        let source_file = self.bots_dir.join(&source_filename);
        fs::write(&source_file, source_code)?;
        let bot = Bot::new(name, source_filename, language);
        bot.save(self.pool.clone()).await?;
        Ok(())
    }

    pub async fn remove_bot(&self, id: uuid::Uuid) -> Result<(), RemoveBotError> {
        if let Some(bot) = Bot::find_by_id(&id, self.pool.clone()).await? {
            let source_file_name = format!("{}.{}", bot.name, bot.language.file_extension());
            let source_file = self.bots_dir.join(source_file_name);
            fs::remove_file(source_file)?;
            bot.delete(self.pool.clone()).await?;
            Ok(())
        } else {
            Err(RemoveBotError::NotFound)
        }
    }
}
