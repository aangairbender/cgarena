use crate::domain::Match;

pub trait MatchFilter {
    fn matches(&self, m: &Match) -> bool;
}

pub mod filters {
    use crate::domain::Match;
    use crate::statistics::match_filter::MatchFilter;

    pub struct All;

    impl MatchFilter for All {
        fn matches(&self, _: &Match) -> bool {
            true
        }
    }
}
