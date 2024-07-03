use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Represents a bot in arena
// This does not have "status" field, because status depends on workers
// and would be determined in the runtime.
#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: i32,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub rating: Rating,
    pub created_at: DateTime<Utc>,
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

/// Represents finished match
/// This should not be created until match result is known
#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: i32,
    pub seed: i32,
    pub bot_ids: Vec<i32>,
    pub ranks: Vec<usize>,
    pub errors: Vec<bool>,
}