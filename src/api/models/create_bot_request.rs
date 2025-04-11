use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateBotRequest {
    pub name: String,
    pub source_code: String,
    pub language: String,
}
