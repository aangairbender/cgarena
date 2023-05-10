use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: Uuid,
    pub seed: i32,
    pub status: Status,
    pub bot_ids: Vec<i32>,
    pub scores: HashMap<i32, f32>,
}

#[derive(Deserialize)]
pub struct NewMatch {
    pub seed: i32,
    pub bot_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Completed,
}
