use std::{path::{Path, PathBuf}, fs, io, rc::Rc};

use thiserror::Error;

use crate::config::Config;

use super::{db::DB, bot_service::BotService};

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/cgarena_config.toml"));
const CONFIG_FILE_NAME: &str = "cgarena_config.toml";
const BOTS_DIR_NAME: &str = "bots";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid config")]
    InvalidConfig(#[from] toml::de::Error),
    #[error(transparent)]
    Other(#[from] io::Error)
}

pub struct ArenaService {
    config: Rc<Config>,
    bot_service: BotService,
}

impl ArenaService {
    pub fn create_new_arena(path: &Path) -> Result<(), io::Error> {
        fs::create_dir(path)?;

        let config_file_path = Self::config_file_path(path);
        std::fs::write(config_file_path, DEFAULT_CONFIG_CONTENT)?;

        let bots_dir_path = Self::bots_dir_path(path);
        fs::create_dir(bots_dir_path)?;

        Ok(())
    }

    pub fn new(path: &Path) -> Result<Self, Error> {
        let config_file_path = Self::config_file_path(path);
        let config_content = fs::read_to_string(config_file_path)?;
        match toml::from_str::<Config>(&config_content) {
            Ok(config) => {
                let config = Rc::new(config);
                let db = DB::open(path);
                let bot_service = BotService::new(Self::bots_dir_path(path), config.clone(), &db);
                Ok(Self { config, bot_service })
            },
            Err(e) => Err(Error::InvalidConfig(e))
        }
    }

    fn config_file_path(path: &Path) -> PathBuf {
        path.join(CONFIG_FILE_NAME)
    }

    fn bots_dir_path(path: &Path) -> PathBuf {
        path.join(BOTS_DIR_NAME)
    }
}

#[cfg(test)]
mod test {
    use crate::config::Config;

    use super::*;

    #[test]
    fn default_config_can_be_deserialized() {
        let config = toml::from_str::<Config>(DEFAULT_CONFIG_CONTENT);
        assert!(config.is_ok());
    }
}