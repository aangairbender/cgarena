use std::path::PathBuf;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub source_file: PathBuf,
    pub language_name: String,
}

impl Bot {
    pub fn new(name: String, source_file: PathBuf, language_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            source_file,
            language_name,
        }
    }
}
