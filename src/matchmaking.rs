use crate::config::{GameConfig, MatchmakingConfig};
use crate::db::Database;
use crate::domain::{BotId, BotStats};
use crate::match_scheduler::ScheduledMatch;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

const INTERVAL: Duration = Duration::from_millis(500);

pub async fn run(
    scheduled_match_tx: Sender<ScheduledMatch>,
    db: Database,
    game_config: Arc<GameConfig>,
    matchmaking_config: Arc<MatchmakingConfig>,
    token: CancellationToken,
) {
    let mut prepared = VecDeque::new();
    loop {
        tokio::select! {
            _ = token.cancelled() => break,
            permit = scheduled_match_tx.reserve() => {
                let permit = permit.expect("Cannot obtain permit for matchmaking");
                if let Some(scheduled_match) = prepared.pop_front() {
                    permit.send(scheduled_match);
                    continue;
                }

                let bot_stats = db.fetch_bot_stats_all().await;
                let scheduled_matches = schedule_match(&bot_stats, Arc::clone(&game_config), Arc::clone(&matchmaking_config));
                if scheduled_matches.is_empty() {
                    // waiting
                    drop(permit);
                    tokio::time::sleep(INTERVAL).await;
                } else {
                    let mut iter = scheduled_matches.into_iter();
                    let first = iter.next().unwrap();
                    permit.send(first);
                    prepared.extend(iter);
                }
            }
        }
    }
}

fn schedule_match(
    bot_stats: &[(BotId, BotStats)],
    game_config: Arc<GameConfig>,
    matchmaking_config: Arc<MatchmakingConfig>,
) -> Vec<ScheduledMatch> {
    let mut rng = thread_rng();

    if bot_stats.len() < game_config.min_players as usize {
        return vec![];
    }

    let bot_ids_min_matches = bot_stats
        .iter()
        .filter(|(_, stats)| stats.matches_played < matchmaking_config.min_matches as _)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();

    let first_bot_id = if !bot_ids_min_matches.is_empty()
        && rng.gen::<f64>() < matchmaking_config.min_matches_preference
    {
        bot_ids_min_matches[rng.gen_range(0..bot_ids_min_matches.len())]
    } else {
        bot_stats[rng.gen_range(0..bot_stats.len())].0
    };

    let n_players = rng.gen_range(game_config.min_players..=game_config.max_players) as usize;
    let mut players = Vec::with_capacity(n_players);
    players.push(first_bot_id);
    while players.len() < n_players {
        let next_bot_id = loop {
            let candidate_id = bot_stats[rng.gen_range(0..bot_stats.len())].0;
            if !players.contains(&candidate_id) {
                break candidate_id;
            }
        };

        players.push(next_bot_id);
    }
    players.shuffle(&mut rng);
    let scheduled_match = ScheduledMatch {
        seed: rng.gen(),
        bot_ids: players,
    };

    if game_config.symmetric {
        vec![scheduled_match]
    } else {
        scheduled_match.into_permutations()
    }
}
