use serde::{Deserialize, Serialize};

use crate::domain::Match;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    pub fn parse(_filter: &str) -> Result<MatchFilter, anyhow::Error> {
        unimplemented!()
    }
}
