use crate::domain::{BotId, Rating, WinrateStats};
use std::collections::HashMap;

pub trait Algorithm {
    fn supports_multi_team(&self) -> bool;
    fn default_rating(&self) -> Rating;
}

pub trait OnlineAlgorithm: Algorithm {
    fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating>;
}

pub trait BatchAlgorithm: Algorithm {
    fn recalc_batch(
        &self,
        winrate_stats: &HashMap<(BotId, BotId), WinrateStats>,
    ) -> HashMap<BotId, Rating>;
}
