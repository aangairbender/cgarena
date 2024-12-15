// use crate::config::{GameConfig, MatchmakingConfig};
// use crate::db::{BotStats, Database};
// use rand::{thread_rng, Rng};
// use tokio::sync::mpsc::Sender;
// use tokio_util::sync::CancellationToken;
//
// pub struct MatchManager {
//     db: Database,
//     game_config: GameConfig,
//     matchmaking_config: MatchmakingConfig,
//     runner_sender: Sender<ScheduledMatch>,
// }
//
// impl MatchManager {
//     pub fn new(
//         db: Database,
//         game_config: GameConfig,
//         matchmaking_config: MatchmakingConfig,
//         runner_sender: Sender<ScheduledMatch>,
//     ) -> Self {
//         Self {
//             db,
//             game_config,
//             matchmaking_config,
//             runner_sender,
//         }
//     }
//
//     pub async fn run(mut self, token: CancellationToken) {
//         loop {
//             tokio::select! {
//                 _ = token.cancelled() => break,
//                 // _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
//                 //     self.perform_matchmaking().await;
//                 // },
//                 _ = self.perform_matchmaking() => continue,
//             }
//         }
//     }
//
//     async fn perform_matchmaking(&mut self) {
//         let bot_stats = self.db.fetch_bot_stats().await.unwrap();
//         let Some(r#match) = self.schedule_match(&bot_stats) else {
//             return;
//         };
//         self.runner_sender
//             .send(r#match)
//             .await
//             .expect("Can't send match to runner");
//     }
//
//     fn schedule_match(&self, bot_stats: &[BotStats]) -> Option<ScheduledMatch> {
//         let mut rng = thread_rng();
//
//         if bot_stats.len() < self.game_config.min_players as usize {
//             return None;
//         }
//
//         let bot_ids_min_matches = bot_stats
//             .iter()
//             .filter(|&stats| stats.games < self.matchmaking_config.min_matches)
//             .map(|stats| stats.bot_id)
//             .collect::<Vec<_>>();
//
//         let first_bot_id = if !bot_ids_min_matches.is_empty()
//             && rng.gen::<f64>() < self.matchmaking_config.min_matches_preference
//         {
//             bot_ids_min_matches[rng.gen_range(0..bot_ids_min_matches.len())]
//         } else {
//             bot_stats[rng.gen_range(0..bot_stats.len())].bot_id
//         };
//
//         let n_players =
//             rng.gen_range(self.game_config.min_players..=self.game_config.max_players) as usize;
//         let mut players = vec![first_bot_id];
//         while players.len() < n_players {
//             let next_bot_id = loop {
//                 let candidate_id = bot_stats[rng.gen_range(0..bot_stats.len())].bot_id;
//                 if !players.contains(&candidate_id) {
//                     break candidate_id;
//                 }
//             };
//
//             players.push(next_bot_id);
//         }
//
//         Some(ScheduledMatch {
//             seed: rng.gen(),
//             bot_ids: players,
//         })
//     }
// }
//
// pub struct ScheduledMatch {
//     pub seed: i32,
//     pub bot_ids: Vec<i32>,
// }
