use std::path::PathBuf;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use super::Language;

#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub source_file: PathBuf,
    pub language: Language,
}

impl Bot {
    pub fn new(name: String, source_file: PathBuf, language: Language) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            source_file,
            language,
        }
    }
}
