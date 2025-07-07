use crate::config::{GameConfig, MatchmakingConfig};
use crate::domain::BotId;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::{random, rng, Rng};

#[derive(Copy, Clone)]
pub struct Candidate {
    pub id: BotId,
    pub matches_played: u64,
}

pub struct MatchConfig {
    pub bot_ids: Vec<BotId>,
    pub seed: i64,
}

pub fn create_match(
    game_config: &GameConfig,
    matchmaking_config: &MatchmakingConfig,
    candidates: &[Candidate],
) -> Vec<MatchConfig> {
    if candidates.len() < game_config.min_players as usize {
        return vec![];
    }

    let bot_ids = pick_participants(game_config, matchmaking_config, candidates);
    let seed: i64 = random();

    if game_config.symmetric {
        vec![MatchConfig { bot_ids, seed }]
    } else {
        let n = bot_ids.len();
        bot_ids
            .into_iter()
            .permutations(n)
            .map(|perm| MatchConfig {
                seed,
                bot_ids: perm,
            })
            .collect()
    }
}

fn pick_participants(
    game_config: &GameConfig,
    matchmaking_config: &MatchmakingConfig,
    candidates: &[Candidate],
) -> Vec<BotId> {
    assert!(candidates.len() >= game_config.min_players as usize);

    let mut rng = rng();

    let bot_ids = candidates.iter().map(|c| c.id).collect_vec();

    let bot_ids_min_matches = candidates
        .iter()
        .copied()
        .filter(|c| c.matches_played < matchmaking_config.min_matches as _)
        .map(|c| c.id)
        .collect::<Vec<_>>();

    let first_bot_id = if !bot_ids_min_matches.is_empty()
        && rng.random::<f64>() < matchmaking_config.min_matches_preference
    {
        bot_ids_min_matches[rng.random_range(0..bot_ids_min_matches.len())]
    } else {
        bot_ids[rng.random_range(0..bot_ids.len())]
    };

    let n_players = rng.random_range(game_config.min_players..=game_config.max_players) as usize;
    let mut players = Vec::with_capacity(n_players);
    players.push(first_bot_id);
    while players.len() < n_players {
        let next_bot_id = loop {
            let candidate_id = bot_ids[rng.random_range(0..bot_ids.len())];
            if !players.contains(&candidate_id) {
                break candidate_id;
            }
        };

        players.push(next_bot_id);
    }
    players.shuffle(&mut rng);
    players
}
