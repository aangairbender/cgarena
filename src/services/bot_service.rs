use std::{fs, path::{Path, PathBuf}, rc::Rc, io};

use thiserror::Error;

use crate::{config::Config, models::{Bot, Language}};

use super::db::DB;

#[derive(Error, Debug)]
pub enum Error {
    #[error("This language is not supported")]
    UnsupportedLanguage,
    #[error("Bot with the same name already exists")]
    AlreadyExists,
    #[error("Bot with such name cannot be found")]
    NotFound,
    #[error(transparent)]
    IO(#[from] io::Error)
}

pub struct BotService {
    bots_dir: PathBuf,
    config: Rc<Config>,
    db: Rc<DB>,
}

impl BotService {
    pub fn new(bots_dir: PathBuf, config: Rc<Config>, db: Rc<DB>) -> Self {
        Self { bots_dir, config, db }
    }

    pub fn add_bot(
        &self,
        name: String,
        source_file: String,
        language_name: Option<String>,
    ) -> Result<Bot, Error> {
        let language = self.detect_language(&source_file, language_name.as_deref())    
            .ok_or(Error::UnsupportedLanguage)?;

        if self.already_exists(&name) {
            return Err(Error::AlreadyExists);
        }
        let source_code_file = self.bots_dir.join(format!("{}.{}", &name, &language.file_extension));
        fs::copy(source_file, &source_code_file)?;

        // super::exec::build_source_code(&name, source_code_file.to_str().unwrap(), language)
        //     .unwrap_or_else(|e| {
        //         fs::remove_dir_all(bot_dir).expect("can't remove bot dir");
        //         eprintln!("code should be without compile errors, but there are some:");
        //         eprintln!("{}", e);
        //         panic!();
        //     });

        let bot = Bot::new(name, language.name.clone(), source_code_file);
        self.db.insert_bot(bot.clone());
        Ok(bot)
    }

    fn already_exists(&self, name: &str) -> bool {
        self.bots_dir.read_dir().map(|iter|
            iter.filter_map(|v| v.ok())
                .any(|f| f.file_name().eq_ignore_ascii_case(name))
        );
    }

    pub fn remove_bot(&self, name: &str) -> Result<(), Error> {
        if let Some(bot) = self.db.fetch_bots().into_iter().find(|b| b.name == name) {
            self.db.delete_bot(name);

            fs::remove_file(bot.source_file)?;
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    pub fn list_bots(self) -> impl Iterator<Item = Bot> {
        self.db.fetch_bots().into_iter()
    }

    fn detect_language(&self, source_file: &str, language_name: Option<&str>) -> Option<&Language> {
        let language_by_extension = |e| {
            self.config
                .languages
                .iter()
                .find(|lang| lang.file_extension == e)
        };
        let language_by_name = |e| self.config.languages.iter().find(|lang| lang.name == e);

        let file_language = Path::new(&source_file)
            .extension()
            .and_then(|e| e.to_str())
            .and_then(language_by_extension);
        
        language_name
            .and_then(language_by_name)
            .or(file_language)
    }
}
