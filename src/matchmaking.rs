use std::collections::HashMap;

use crate::config::{GameConfig, MatchmakingConfig};
use crate::domain::BotId;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::{random, rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "algorithm", rename_all = "snake_case")]
pub enum MatchmakingAlgorithmConfig {
    V1(MatchmakingAlgorithmV1Config),
    V2(MatchmakingAlgorithmV2Config),

    // The "Fallback" for old configs
    #[serde(untagged)]
    Legacy(MatchmakingAlgorithmV1Config),
}

#[derive(Serialize, Deserialize)]
pub struct MatchmakingAlgorithmV1Config {
    pub min_matches: u64,
    pub min_matches_preference: f64,
}

#[derive(Serialize, Deserialize)]
pub struct MatchmakingAlgorithmV2Config {
    pub min_matches_against_best: Option<u64>,
    pub min_matches_per_pair: u64,
    pub max_matches: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: BotId,
    pub rating: f64,
    // the numbers below should include queued matches
    pub matches_total: u64,
    pub matches_vs: HashMap<BotId, u64>,
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

    let n_players = rng().random_range(game_config.min_players..=game_config.max_players) as usize;

    let bot_ids = match &matchmaking_config.algorithm {
        MatchmakingAlgorithmConfig::V1(matchmaking_algorithm_v1_config) => pick_participants_v1(n_players, matchmaking_algorithm_v1_config, candidates),
        MatchmakingAlgorithmConfig::V2(matchmaking_algorithm_v2_config) => pick_participants_v2(n_players, matchmaking_algorithm_v2_config, candidates),
        MatchmakingAlgorithmConfig::Legacy(matchmaking_algorithm_v1_config) => pick_participants_v1(n_players, matchmaking_algorithm_v1_config, candidates),
    };

    let Some(bot_ids) = bot_ids else {
        return vec![];
    };

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

fn pick_participants_v1(
    n_players: usize,
    matchmaking_config: &MatchmakingAlgorithmV1Config,
    candidates: &[Candidate],
) -> Option<Vec<BotId>> {
    let min_matches_preference = matchmaking_config.min_matches_preference.clamp(0.0, 1.0);

    let mut rng = rng();

    let bot_ids = candidates.iter().map(|c| c.id).collect_vec();

    let bot_ids_min_matches = candidates
        .iter()
        .filter(|c| c.matches_total < matchmaking_config.min_matches as _)
        .map(|c| c.id)
        .collect::<Vec<_>>();

    let first_bot_id = if !bot_ids_min_matches.is_empty()
        && rng.random::<f64>() < min_matches_preference
    {
        bot_ids_min_matches[rng.random_range(0..bot_ids_min_matches.len())]
    } else {
        bot_ids[rng.random_range(0..bot_ids.len())]
    };

    let mut players = Vec::with_capacity(n_players);
    players.push(first_bot_id);
    backfill_random(n_players, &mut players, candidates);
    players.shuffle(&mut rng);
    Some(players)
}

fn pick_participants_v2(
    n_players: usize,
    matchmaking_config: &MatchmakingAlgorithmV2Config,
    candidates: &[Candidate],
) -> Option<Vec<BotId>> {
    let mut rng = rng();

    // 1. if some bot has less games vs best than `min_matches_against_best` then create a game with that bot and best.

    let best_bot_ids = candidates.iter()
        .sorted_by(|a, b| a.rating.total_cmp(&b.rating).reverse())
        .take(2)
        .map(|c| c.id)
        .collect_vec();

    assert_eq!(best_bot_ids.len(), 2, "this function should not be called with less than 2 candidates");
    let best_bot_id_for = |id: BotId| -> BotId {
        if best_bot_ids[0] == id {
            best_bot_ids[1]
        } else {
            best_bot_ids[0]
        }
    };

    let can_match_top2 = {
        let top1 = candidates.iter().find(|c| c.id == best_bot_ids[0]).unwrap();
        let top2_cnt = *top1.matches_vs.get(&best_bot_ids[1]).unwrap_or(&0);
        top2_cnt < matchmaking_config.min_matches_against_best.unwrap_or(0)
    };

    let prio_vs_best = if can_match_top2 {
        Some((best_bot_ids[0], best_bot_ids[1]))
    } else {
        candidates.iter()
            .map(|c| {
                let best_for_me = best_bot_id_for(c.id);
                let (opp, cnt) = c.matches_vs.iter()
                    .find(|&(opp, _)| *opp == best_for_me)
                    .unwrap();
                (c.id, (*opp, *cnt))
            })
            .min_by_key(|(_, (_, cnt))| *cnt)
            .filter(|(_, (_, cnt))| *cnt < matchmaking_config.min_matches_against_best.unwrap_or(0))
            .map(|(bot_a, (bot_b, _))| (bot_a, bot_b))
    };


    // bot with lowest matches playest against some other bot
    let pair_lower_than_min = candidates.iter()
        .map(|c| (c.id, c.matches_vs.iter().map(|(opp, cnt)| (*opp, *cnt)).min_by_key(|(_, cnt)| *cnt).unwrap()))
        .min_by_key(|(_, (_, cnt))| *cnt)
        .filter(|(_, (_, cnt))| *cnt < matchmaking_config.min_matches_per_pair)
        .map(|(bot_a, (bot_b, _))| (bot_a, bot_b));

    let random_lower_than_max = {
        let bot_ids_with_not_enough_matches = candidates.iter()
            .filter(|c| c.matches_total < matchmaking_config.max_matches.unwrap_or(u64::MAX))
            .map(|c| c.id)
            .collect_vec();
        let cnt = bot_ids_with_not_enough_matches.len();
        if cnt == 0 {
            None
        } else {
            let index = rng.random_range(0..cnt);
            Some(bot_ids_with_not_enough_matches[index])
        }
    };

    let (first_bot_id, second_bot_id) = if let Some((a, b)) = prio_vs_best {
        assert_ne!(a, b, "first and second bot must be different");
        (a, Some(b))
    } else if let Some((a, b)) = pair_lower_than_min {
        assert_ne!(a, b, "first and second bot must be different");
        (a, Some(b))
    } else if let Some(a) = random_lower_than_max {
        (a, None)
    } else {
        return None;
    };

    let mut players = Vec::with_capacity(n_players);
    players.push(first_bot_id);
    players.extend(second_bot_id);
    backfill_random(n_players, &mut players, candidates);
    players.shuffle(&mut rng);
    Some(players)
}

fn backfill_random(
    n_players: usize,
    players: &mut Vec<BotId>,
    candidates: &[Candidate],
) {
    let mut rng = rng();
    while players.len() < n_players {
        let next_bot_id = loop {
            let candidate_id = candidates[rng.random_range(0..candidates.len())].id;
            if !players.contains(&candidate_id) {
                break candidate_id;
            }
        };

        players.push(next_bot_id);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn v2_prio_best() {
        let config = MatchmakingAlgorithmV2Config {
            min_matches_against_best: Some(3),
            min_matches_per_pair: 5,
            max_matches: None,
        };

        let candidates = vec![
            Candidate { id: 1.into(), rating: 2.0, matches_total: 5, matches_vs: [(2.into(), 3), (3.into(), 2)].into() },
            Candidate { id: 2.into(), rating: 1.0, matches_total: 3, matches_vs: [(1.into(), 3), (3.into(), 0)].into() },
            Candidate { id: 3.into(), rating: 1.0, matches_total: 2, matches_vs: [(1.into(), 2), (2.into(), 0)].into() },
        ];

        let bot_ids: Vec<i64> = pick_participants_v2(2, &config, &candidates)
            .unwrap()
            .into_iter()
            .map(|id| id.into())
            .sorted()
            .collect();

        assert_eq!(&bot_ids, &[1, 3]);
    }

    #[test]
    fn v2_lower_pair() {
        let config = MatchmakingAlgorithmV2Config {
            min_matches_against_best: None,
            min_matches_per_pair: 5,
            max_matches: None,
        };

        let candidates = vec![
            Candidate { id: 1.into(), rating: 2.0, matches_total: 5, matches_vs: [(2.into(), 3), (3.into(), 2)].into() },
            Candidate { id: 2.into(), rating: 1.0, matches_total: 3, matches_vs: [(1.into(), 3), (3.into(), 0)].into() },
            Candidate { id: 3.into(), rating: 1.0, matches_total: 2, matches_vs: [(1.into(), 2), (2.into(), 0)].into() },
        ];

        let bot_ids: Vec<i64> = pick_participants_v2(2, &config, &candidates)
            .unwrap()
            .into_iter()
            .map(|id| id.into())
            .sorted()
            .collect();

        assert_eq!(&bot_ids, &[2, 3]);
    }
}