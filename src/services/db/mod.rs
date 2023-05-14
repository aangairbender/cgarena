mod json_db;

use self::json_db::JsonDB;
use crate::models::{Bot, Match};
use std::path::{Path, PathBuf};

pub struct DB {
    bots: JsonDB<Bot>,
    matches: JsonDB<Match>,
}

impl DB {
    pub fn open(arena_root: &Path) -> Self {
        Self {
            bots: JsonDB::new(Self::model_db_file_path(arena_root, "bots")),
            matches: JsonDB::new(Self::model_db_file_path(arena_root, "matches")),
        }
    }

    pub fn insert_bot(&self, bot: Bot) {
        self.bots
            .modify(|bots| {
                bots.push(bot);
            })
            .expect("Should be possible to add bot");
    }

    pub fn delete_bot(&self, name: &str) {
        self.bots
            .modify(|bots| {
                bots.retain(|b| b.name != name);
            })
            .expect("Should be possible to remove bot");
    }

    pub fn fetch_bots(&self) -> Vec<Bot> {
        self.bots.load().expect("Should be possible to load bots")
    }

    pub fn insert_match(&self, m: Match) {
        self.matches
            .modify(|matches| {
                matches.push(m);
            })
            .expect("Should be possible to add match");
    }

    fn model_db_file_path(arena_root: &Path, model: &str) -> PathBuf {
        arena_root.join(model).join(format!("{}.db", model))
    }
}
