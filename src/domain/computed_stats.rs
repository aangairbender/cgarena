use crate::domain::{BotId, Match, Rating};
use crate::ranking::Ranker;
use std::collections::HashMap;

#[derive(Default)]
pub struct ComputedStats {
    pub ratings: HashMap<BotId, Rating>,
    pub matches_played: HashMap<BotId, u64>,
    pub matches_with_error: HashMap<BotId, u64>,
}

impl ComputedStats {
    pub fn clear(&mut self) {
        *self = Default::default();
    }

    pub fn recalc_after_match(&mut self, ranker: &Ranker, m: &Match) {
        // rating
        ranker.recalc_rating(&mut self.ratings, m);

        // matches_played and matches_with_error
        for p in &m.participants {
            self.matches_played
                .entry(p.bot_id)
                .and_modify(|w| *w += 1)
                .or_insert(1);

            if p.error {
                self.matches_with_error
                    .entry(p.bot_id)
                    .and_modify(|w| *w += 1)
                    .or_insert(1);
            }
        }
    }

    pub fn rating(&self, id: BotId) -> Option<Rating> {
        self.ratings.get(&id).cloned()
    }

    pub fn matches_played(&self, id: BotId) -> u64 {
        self.matches_played.get(&id).copied().unwrap_or_default()
    }

    pub fn matches_with_error(&self, id: BotId) -> u64 {
        self.matches_with_error
            .get(&id)
            .copied()
            .unwrap_or_default()
    }
}
