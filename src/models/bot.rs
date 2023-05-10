use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub source_code: String,
    pub language_tag: String,
}

#[derive(Deserialize)]
pub struct NewBot {
    pub name: String,
    pub description: String,
    pub source_code: String,
    pub language: String,
}
