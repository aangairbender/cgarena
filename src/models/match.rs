use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: Uuid,
    pub status: Status,
    pub seed: i32,
    pub bot_ids: Vec<Uuid>,
    pub rotation_offset: u8,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Pending,
    Assigned { worker_id: Uuid },
    Completed { bot_scores: HashMap<Uuid, f32> },
}
