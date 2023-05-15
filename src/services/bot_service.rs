use std::{fs, path::Path};

use crate::{config::Config, models::{Bot, Language}};

use super::db::DB;

pub struct BotService<'a> {
    config: &'a Config,
    db: &'a DB,
}

impl<'a> BotService<'a> {
    pub fn new(config: &'a Config, db: &'a DB) -> Self {
        Self { config, db }
    }

    pub fn add_bot(
        &'a self,
        name: String,
        source_file: String,
        language_name: Option<String>,
    ) -> Result<Bot, Error> {
        let language = self.detect_language(&source_file, language_name.as_deref())    
            .ok_or(Error::UnsupportedLanguage)?;

        let bot_dir = std::env::current_dir()
            .unwrap()
            .join("bots")
            .join(&name);
        fs::create_dir(&bot_dir)
            .expect("Should create a directory for a new bot");
        let source_code_file = bot_dir.join(format!("source.{}", &language.file_extension));
        fs::copy(source_file, &source_code_file)
            .expect("Should copy source code to the dedicated folder");

        super::exec::build_source_code(&name, source_code_file.to_str().unwrap(), language)
            .unwrap_or_else(|e| {
                fs::remove_dir_all(bot_dir).expect("can't remove bot dir");
                eprintln!("code should be without compile errors, but there are some:");
                eprintln!("{}", e);
                panic!();
            });

        let bot = Bot::new(name, language.name.clone());
        self.db.insert_bot(bot.clone());
        Ok(bot)
    }

    pub fn remove_bot(&'a self, name: &str) {
        self.db.delete_bot(name);

        let bot_dir = std::env::current_dir().unwrap().join("bots").join(name);
        fs::remove_dir_all(bot_dir).expect("can't remove bot dir");
    }

    pub fn list_bots(&'a self) -> impl Iterator<Item = Bot> {
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

#[derive(Debug)]
pub enum Error {
    SourceNotFound,
    UnsupportedLanguage,
    BuildError,
}