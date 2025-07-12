use std::str::FromStr;

use crate::domain::Match;

pub enum MatchFilter {
    Empty,
    Eq(String, String),
}

impl MatchFilter {
    pub fn matches(&self, _m: &Match) -> bool {
        match self {
            MatchFilter::Empty => true,
            _ => unimplemented!(),
        }
    }
}

impl FromStr for MatchFilter {
    type Err = anyhow::Error;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        Ok(MatchFilter::Empty)
    }
}

impl ToString for MatchFilter {
    fn to_string(&self) -> String {
        "".to_string()
    }
}
