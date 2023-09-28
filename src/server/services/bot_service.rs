use std::{
    fs,
    path::{Path, PathBuf},
};

use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, Set};

use crate::server::{entities::bot, enums::Language};

pub struct BotService {
    bots_dir: PathBuf,
    db: DatabaseConnection,
}

#[derive(thiserror::Error, Debug)]
pub enum AddBotError {
    #[error("Bot with the same name already exists")]
    DuplicateName,
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] DbErr),
}

#[derive(thiserror::Error, Debug)]
pub enum RemoveBotError {
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    DB(#[from] DbErr),
}

#[derive(thiserror::Error, Debug)]
pub enum ListBotsError {
    #[error(transparent)]
    DB(#[from] DbErr),
}

impl BotService {
    pub fn new(bots_dir: &Path, db: DatabaseConnection) -> Self {
        Self {
            bots_dir: bots_dir.to_owned(),
            db,
        }
    }

    pub async fn add_bot(
        &self,
        name: String,
        source_code: String,
        language: Language,
    ) -> Result<(), AddBotError> {
        let duplicate = bot::Entity::find()
            .filter(bot::Column::Name.eq(&name))
            .one(&self.db)
            .await?;

        if duplicate.is_some() {
            return Err(AddBotError::DuplicateName);
        }

        let source_filename = format!("{}.{}", name, language.file_extension());
        let source_file = self.bots_dir.join(&source_filename);
        fs::write(&source_file, source_code)?;

        let bot = bot::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            name: Set(name),
            source_filename: Set(source_filename),
            language: Set(language),
        };

        bot::Entity::insert(bot).exec_without_returning(&self.db).await?;
        Ok(())
    }

    pub async fn remove_bot(&self, id: uuid::Uuid) -> Result<(), RemoveBotError> {
        let Some(bot) = bot::Entity::find_by_id(id).one(&self.db).await? else {
            return Err(RemoveBotError::NotFound)
        };
        let source_file_name = format!("{}.{}", bot.name, bot.language.file_extension());
        let source_file = self.bots_dir.join(source_file_name);
        fs::remove_file(source_file)?;
        bot.delete(&self.db).await?;
        Ok(())
    }

    pub async fn list_bots(&self) -> Result<Vec<bot::Model>, ListBotsError> {
        let bots = bot::Entity::find().all(&self.db).await?;
        Ok(bots)
    }
}
