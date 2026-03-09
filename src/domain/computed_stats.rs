use itertools::Itertools;

use crate::domain::{BotId, Match, Rating};
use crate::ranking::{Ranker, RankingStrategyKind};
use std::collections::{HashMap, VecDeque};

#[derive(Default, Clone)]
pub struct ComputedStats {
    ratings: HashMap<BotId, Rating>,
    winrate_stats: HashMap<(BotId, BotId), WinrateStats>,
    matches_with_error: HashMap<BotId, u64>,
    total_matches: u64,
    example_seeds: VecDeque<i64>,
}

const EXAMPLE_SEEDS_LIMIT: usize = 10;

#[derive(Default, Clone)]
pub struct WinrateStats {
    pub wins: u64,
    pub draws: u64,
    pub loses: u64,
}

impl WinrateStats {
    pub fn total(&self) -> u64 {
        self.wins + self.loses + self.draws
    }
}

impl ComputedStats {
    pub fn recalc_after_matches(&mut self, ranker: &Ranker, matches: &[&Match]) {
        self.total_matches += matches.len() as u64;

        for &m in matches {
            self.recalc_example_seeds_after_match(m);
            self.recalc_matches_with_error_after_match(m);
            self.recalc_winrate_stats_after_match(m);
        }

        // rating
        match ranker.strategy_kind() {
            RankingStrategyKind::Online => {
                for &m in matches {
                    ranker.recalc_rating(&mut self.ratings, m);
                }
            }
            RankingStrategyKind::Batch => {
                self.ratings = ranker.recalc_rating_batch(&self.winrate_stats)
            }
        }
    }

    fn recalc_example_seeds_after_match(&mut self, m: &Match) {
        if !self.example_seeds.contains(&m.seed) {
            self.example_seeds.push_front(m.seed);
            while self.example_seeds.len() > EXAMPLE_SEEDS_LIMIT {
                self.example_seeds.pop_back();
            }
        }
    }

    fn recalc_matches_with_error_after_match(&mut self, m: &Match) {
        for p in &m.participants {
            if p.error {
                self.matches_with_error
                    .entry(p.bot_id)
                    .and_modify(|w| *w += 1)
                    .or_insert(1);
            }
        }
    }

    fn recalc_winrate_stats_after_match(&mut self, m: &Match) {
        for (p1, p2) in m
            .participants
            .iter()
            .cartesian_product(m.participants.iter())
        {
            if p1.bot_id == p2.bot_id {
                continue;
            }
            let entry = self
                .winrate_stats
                .entry((p1.bot_id, p2.bot_id))
                .or_default();

            match p1.rank.cmp(&p2.rank) {
                std::cmp::Ordering::Less => entry.wins += 1,
                std::cmp::Ordering::Equal => entry.draws += 1,
                std::cmp::Ordering::Greater => entry.loses += 1,
            }
        }
    }

    pub fn rating(&self, id: BotId) -> Option<Rating> {
        self.ratings.get(&id).cloned()
    }

    pub fn matches_played(&self, id: BotId) -> u64 {
        self.winrate_stats
            .iter()
            .filter(|(k, _)| k.0 == id)
            .map(|(_, v)| v.total())
            .sum()
    }

    pub fn matches_played_vs(&self, id: BotId, opp: BotId) -> u64 {
        self.winrate_stats
            .iter()
            .find(|(k, _)| k.0 == id && k.1 == opp)
            .map(|(_, v)| v.total())
            .unwrap_or(0)
    }

    pub fn winrate_stats_snapshot(&self) -> HashMap<(BotId, BotId), WinrateStats> {
        self.winrate_stats.clone()
    }

    pub fn matches_with_error(&self, id: BotId) -> u64 {
        self.matches_with_error
            .get(&id)
            .copied()
            .unwrap_or_default()
    }

    pub fn total_matches(&self) -> u64 {
        self.total_matches
    }

    pub fn example_seeds(&self) -> Vec<i64> {
        self.example_seeds.iter().cloned().collect()
    }
}
