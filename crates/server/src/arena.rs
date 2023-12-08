use anyhow::{bail, Ok};
use config::Config;
use entity::{
    bot,
    r#match::{self, MatchStatus},
};
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait, Set,
};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;
use worker::{Job, Worker};

pub struct Arena {
    config: Arc<Config>,
    db: DatabaseConnection,
    match_queue_rx: mpsc::UnboundedReceiver<i32>,
    workers: Vec<Worker>,
    last_selected_worker_index: usize,
}

impl Arena {
    pub fn new(
        config: Arc<Config>,
        db: DatabaseConnection,
        match_queue_rx: mpsc::UnboundedReceiver<i32>,
    ) -> Self {
        let workers = if let Some(w) = &config.embedded_worker {
            vec![Worker::new(w.clone()).unwrap()]
        } else {
            vec![]
        };
        Self {
            config,
            db,
            match_queue_rx,
            workers,
            last_selected_worker_index: 0,
        }
    }

    pub async fn launch(mut self) {
        while let Some(match_id) = self.match_queue_rx.recv().await {
            self.organize_match(match_id)
                .await
                .expect("cannot organize match");
        }
    }

    async fn organize_match(&mut self, match_id: i32) -> Result<(), anyhow::Error> {
        let r#match = r#match::Entity::find_by_id(match_id).one(&self.db).await?;

        let Some(r#match) = r#match else {
            bail!("Organized match does not exist int db, skipping");
        };

        let mut participations = r#match
            .find_related(entity::participation::Entity)
            .all(&self.db)
            .await?;

        participations.sort_by_key(|p| p.index);

        let best_worker_index = self.choose_worker_for(&r#match);

        let mut bots = Vec::with_capacity(participations.len());
        for participation in &participations {
            let bot = bot::Entity::find_by_id(participation.bot_id)
                .one(&self.db)
                .await?;

            let Some(bot) = bot else {
                bail!("Organized match participant does not exist in db");
            };
            bots.push(bot);
        }

        let job = Job {
            r#match: r#match.clone(),
            bots,
        };

        let res = self.workers[best_worker_index].run(job).await?;

        for participation in participations {
            let index = participation.index as usize;
            let mut participation = participation.into_active_model();
            participation.score = Set(Some(res.scores[index]));
            participation.update(&self.db).await?;
        }

        let mut r#match = r#match.into_active_model();
        r#match.status = Set(MatchStatus::Finished);
        r#match.update(&self.db).await?;

        Ok(())
    }

    fn choose_worker_for(&mut self, _match: &r#match::Model) -> usize {
        // TODO: implement smarter strategy
        self.last_selected_worker_index += 1;
        self.last_selected_worker_index %= self.workers.len();
        self.last_selected_worker_index
    }
}
