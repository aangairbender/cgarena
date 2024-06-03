use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use crate::config::Config;
use crate::{model::*, persistence};

#[derive(thiserror::Error, Debug)]
pub enum ArenaError {
    #[error("Already exists")]
    AlreadyExists,
    #[error("Not found")]
    NotFound,
}

pub struct Arena {
    path: PathBuf,
    config: Config,
    bots: Vec<Bot>,
}

impl Arena {
    pub fn new(path: PathBuf, config: Config) -> Self {
        let bots = persistence::load_bots(&path).unwrap();
        Self {
            path,
            config,
            bots
        }
    }

    fn bot_index_by_name(&self, name: &str) -> Option<usize> {
        self.bots.iter().enumerate().find(|&(_, bot)| bot.name == name).map(|w| w.0)
    }

    pub fn add_bot(&mut self, name: String, source_code: String, language: String) -> Result<(), ArenaError> {
        if self.bot_index_by_name(&name).is_some() {
            return Err(ArenaError::AlreadyExists);
        }

        let bot = Bot {
            id: Uuid::new_v4(),
            name,
            source_code,
            language,
            status: BotStatus::Pending,
            rating: Rating::default(),
            created_at: Utc::now(),
        };
        self.bots.push(bot);
        Ok(())
    }

    pub fn remove_bot(&mut self, name: &str) -> Result<(), ArenaError> {
        match self.bot_index_by_name(name) {
            Some(index) => {
                self.bots.swap_remove(index);
                Ok(())
            },
            None => Err(ArenaError::NotFound),
        }
    }

    pub fn rename_bot(&mut self, old_name: &str, new_name: String) -> Result<(), ArenaError> {
        if old_name == new_name {
            return Ok(())
        }
        let Some(bot_index) = self.bot_index_by_name(old_name) else {
            return Err(ArenaError::NotFound);
        };
        if self.bot_index_by_name(&new_name).is_some() {
            return Err(ArenaError::AlreadyExists);
        }
        self.bots[bot_index].name = new_name;
        Ok(())
    }

    // pub async fn launch(mut self) {
    //     while let Some(match_id) = self.match_queue_rx.recv().await {
    //         self.organize_match(match_id)
    //             .await
    //             .expect("cannot organize match");
    //     }
    // }

    // async fn organize_match(&mut self, match_id: i32) -> Result<(), anyhow::Error> {
    //     let r#match = r#match::Entity::find_by_id(match_id).one(&self.db).await?;

    //     let Some(r#match) = r#match else {
    //         bail!("Organized match does not exist int db, skipping");
    //     };

    //     let mut participations = r#match
    //         .find_related(entity::participation::Entity)
    //         .all(&self.db)
    //         .await?;

    //     participations.sort_by_key(|p| p.index);

    //     let best_worker_index = self.choose_worker_for(&r#match);

    //     let mut bots = Vec::with_capacity(participations.len());
    //     for participation in &participations {
    //         let bot = bot::Entity::find_by_id(participation.bot_id)
    //             .one(&self.db)
    //             .await?;

    //         let Some(bot) = bot else {
    //             bail!("Organized match participant does not exist in db");
    //         };
    //         bots.push(bot);
    //     }

    //     let job = Job {
    //         r#match: r#match.clone(),
    //         bots,
    //     };

    //     let res = self.workers[best_worker_index].run(job).await?;

    //     for participation in participations {
    //         let index = participation.index as usize;
    //         let mut participation = participation.into_active_model();
    //         participation.score = Set(Some(res.scores[index]));
    //         participation.update(&self.db).await?;
    //     }

    //     let mut r#match = r#match.into_active_model();
    //     r#match.status = Set(MatchStatus::Finished);
    //     r#match.update(&self.db).await?;

    //     Ok(())
    // }

    // fn choose_worker_for(&mut self, _match: &r#match::Model) -> usize {
    //     // TODO: implement smarter strategy
    //     self.last_selected_worker_index += 1;
    //     self.last_selected_worker_index %= self.workers.len();
    //     self.last_selected_worker_index
    // }
}
