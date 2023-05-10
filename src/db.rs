use dotenvy::dotenv;
use std::env;

use crate::models::{Bot, Match};

pub struct DB {
    bots: Vec<Bot>,
    matches: Vec<Match>,
}

impl DB {
    pub fn open() -> Self {
        // dotenv().ok();

        // let _database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        Self {
            bots: Default::default(),
            matches: Default::default()
        }
    }

    pub fn insert_bot(&mut self, bot: Bot) {
        self.bots.push(bot);
    }

    pub fn delete_bot(&mut self, name: &str) {
        self.bots.retain(|b| b.name != name);
    }

    pub fn fetch_bots(&self) -> Vec<Bot> {
        self.bots.clone()
    }

    pub fn insert_match(&mut self, m: Match) {
        self.matches.push(m);
    }
}
