use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: i32,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub status: BotStatus,
    pub rating: Rating,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotStatus {
    Pending,
    Building,
    Ready,
}

#[derive(Serialize, Deserialize)]
pub struct Rating {
    pub mu: f64,
    pub sigma: f64,
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            mu: 25.0,
            sigma: 25.0 / 3.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: i32,
    pub seed: i32,
    pub status: MatchStatus,
    pub bot_ids: Vec<i32>,
    pub ranks: Vec<usize>,
    pub errors: Vec<bool>,
}

#[derive(Serialize, Deserialize)]
pub enum MatchStatus {
    InQueue,
    Running,
    Finished,
}
