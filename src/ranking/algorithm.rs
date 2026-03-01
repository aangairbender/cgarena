use crate::domain::Rating;

pub trait Algorithm {
    fn supports_multi_team(&self) -> bool;
    fn default_rating(&self) -> Rating;
    fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating>;
}
