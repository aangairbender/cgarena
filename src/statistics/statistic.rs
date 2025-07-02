use crate::domain::{BotId, Match, Rating};
use crate::ranking::Ranker;
use crate::statistics::computed_stats::ComputedStats;
use crate::statistics::match_filter::{filters, MatchFilter};
use std::sync::Arc;

pub struct Statistic {
    filter: Arc<dyn MatchFilter + Send + Sync>,
    ranker: Ranker,
    stats: ComputedStats,
}

impl Statistic {
    pub fn new_without_filter(ranker: Ranker) -> Statistic {
        Self {
            filter: Arc::new(filters::All),
            ranker,
            stats: Default::default(),
        }
    }

    pub fn process(&mut self, m: &Match) {
        if !self.filter.matches(m) {
            return;
        }

        self.stats.recalc_after_match(&self.ranker, m);
    }

    pub fn reset(&mut self) {
        self.stats.clear();
    }

    pub fn rating(&self, id: BotId) -> Rating {
        self.stats
            .ratings
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.ranker.default_rating())
    }

    pub fn matches_played(&self, id: BotId) -> u64 {
        self.stats
            .matches_played
            .get(&id)
            .copied()
            .unwrap_or_default()
    }

    pub fn matches_with_error(&self, id: BotId) -> u64 {
        self.stats
            .matches_with_error
            .get(&id)
            .copied()
            .unwrap_or_default()
    }
}
