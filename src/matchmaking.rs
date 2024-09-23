use rand::{thread_rng, Rng};

use crate::{config::{GameConfig, MatchmakingConfig}, db::Database};

pub struct ScheduledMatch {
    seed: i32,
    bot_ids: Vec<i32>,
}

pub async fn schedule_match(
    game_config: GameConfig,
    matchmaking_config: MatchmakingConfig,
    db: Database,
) -> Option<ScheduledMatch> {
    let mut rng = thread_rng();

    let bot_stats = db.fetch_bot_stats().await.unwrap();
        
    if bot_stats.len() < game_config.min_players as usize {
        return None;
    }

    let bot_ids_min_matches = bot_stats.iter()
        .filter(|&stats| stats.games < matchmaking_config.min_matches)
        .map(|stats| stats.bot_id)
        .collect::<Vec<_>>();

    let first_bot_id = if !bot_ids_min_matches.is_empty() && rng.gen::<f64>() < matchmaking_config.min_matches_preference {
        bot_ids_min_matches[rng.gen_range(0..bot_ids_min_matches.len())]
    } else {
        bot_stats[rng.gen_range(0..bot_stats.len())].bot_id
    };

    let n_players = rng.gen_range(game_config.min_players..=game_config.max_players) as usize;
    let mut players = vec![first_bot_id];
    while players.len() < n_players {
        let next_bot_id = loop {
            let candidate_id = bot_stats[rng.gen_range(0..bot_stats.len())].bot_id;
            if !players.contains(&candidate_id) {
                break candidate_id;
            }
        };

        players.push(next_bot_id);
    }

    Some(ScheduledMatch {
        seed: rng.gen(),
        bot_ids: players,
    })
}