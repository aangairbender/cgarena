use std::{path::{Path, PathBuf}, fs, io, sync::Arc};

use thiserror::Error;

use crate::{server::{config::{Config, ServerConfig}, workers::EmbeddedWorker}, db::{memory_db::MemoryDB, DB}, models::{Bot, Language}};

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cgarena_config.toml"));

const CONFIG_FILE_NAME: &str = "cgarena_config.toml";
const BOTS_DIR_NAME: &str = "bots";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid config")]
    InvalidConfig(#[from] toml::de::Error),
    #[error(transparent)]
    IO(#[from] io::Error)
}

pub struct ArenaService {
    path: PathBuf,
    config: Config,
    worker: EmbeddedWorker, // TODO: replace with channel
    bots: MemoryDB<Bot>,
}

/// Service which manages bots.
/// Can add and remove bots.
/// Should not do any extra work, SRP!
/// Bots are persisted and events are sent to other services
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
        let config = Self::load_config(path)?;
        let worker = EmbeddedWorker::new(config.server.embedded_worker_threads);
        Ok(Self { path: path.to_owned(), config, worker, bots: Default::default() })
    }

    pub fn server_config(&self) -> &ServerConfig {
        &self.config.server
    }

    pub async fn add_bot(&self, name: String, source_code: String, language: Language) -> Result<(), Error> {
        let source_file_name = format!("{}.{}", name, language.file_extension());
        let source_file = Self::bots_dir_path(&self.path).join(source_file_name);
        fs::write(&source_file, source_code)?;
        let bot = Bot::new(name, source_file, language);
        self.bots.put(bot.id, bot);
        Ok(())
    }

    pub async fn remove_bot(&self, name: String) -> Result<(), Error> {
        let source_file_name = format!("{}.{}", name, language.file_extension());
        let source_file = Self::bots_dir_path(&self.path).join(source_file_name);
        fs::remove_file(&source_file)?;
        self.bots.delete(id);
        Ok(())
    }

    fn load_config(path: &Path) -> Result<Config, Error> {
        let config_file_path = Self::config_file_path(path);
        let config_content = fs::read_to_string(config_file_path)?;
        toml::from_str(&config_content)
            .map_err(Error::InvalidConfig)
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
    use tempdir::TempDir;
    use super::*;

    #[test]
    fn new_arena_can_be_created() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        let res = ArenaService::create_new_arena(&path);
        assert!(res.is_ok(), "Arena creation failed {:?}", res.err());
        assert!(path.join("cgarena_config.toml").exists());
        assert!(path.join("bots").is_dir());

        let arena = ArenaService::new(&path);
        assert!(arena.is_ok(), "New arena load failed {:?}", arena.err());
    }
}