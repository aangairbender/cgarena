use std::{path::{PathBuf, Path}, io, fs};

use thiserror::Error;

use crate::models::Language;

pub struct BotService {
    bots_dir: PathBuf,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error)
}

impl BotService {
    pub fn new(bots_dir: &Path) -> Self {
        Self { bots_dir: bots_dir.to_owned() }
    }

    pub async fn add_bot(&self, name: String, source_code: String, language: Language) -> Result<(), Error> {
        let source_filename = format!("{}.{}", name, language.file_extension());
        let source_file = self.bots_dir.join(source_filename);
        fs::write(&source_file, source_code)?;
        let bot = Bot::new(name, source_file, language);
        self.bots.put(bot.id, bot);
        Ok(())
    }

    pub async fn remove_bot(&self, name: String) -> Result<(), Error> {
        // first need to get bot from db
        // then delete file (using language)
        // then delete from db
        let source_file_name = format!("{}.{}", name, language.file_extension());
        let source_file = Self::bots_dir_path(&self.path).join(source_file_name);
        fs::remove_file(&source_file)?;
        self.bots.delete(id);
        Ok(())
    }
}