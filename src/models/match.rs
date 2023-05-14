use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: Uuid,
    pub seed: i32,
    pub status: Status,
    pub bot_ids: Vec<Uuid>,
    pub bot_scores: HashMap<Uuid, f32>,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Completed,
}
