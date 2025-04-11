use serde::Deserialize;

#[derive(Deserialize)]
pub struct RenameBotRequest {
    pub name: String,
}
