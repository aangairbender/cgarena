// use crate::config::RankingConfig;
// use crate::db::Database;
// use crate::domain::BotId;
// use itertools::Itertools;
// use std::collections::HashMap;
// use std::sync::Arc;
// use tracing::warn;
//
// #[derive(Clone)]
// struct Ranking {
//     config: Arc<RankingConfig>,
//     db: Database,
// }
//
// impl Ranking {
//     pub fn new(config: Arc<RankingConfig>, db: Database) -> Self {
//         Self { config, db }
//     }
//
//     pub async fn update_rating(&self, bot_ids: &[BotId], ranks: &[usize]) {
//         let ranks = bot_ids.iter().zip(ranks).collect_vec();
//         let current_ratings = {
//             let mut res = HashMap::new();
//             for &id in bot_ids {
//                 let Some(bot) = self.db.fetch_bot(id).await else {
//                     warn!("Cannot update rating. Bot id {:?} is missing", id);
//                     return;
//                 };
//                 res.insert(id, bot.rating);
//             }
//             res
//         };
//
//         match self.config.as_ref() {
//             &RankingConfig::OpenSkill => {}
//         }
//     }
// }
