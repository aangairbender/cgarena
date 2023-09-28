use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(Some(32))")]
pub enum Language {
    #[sea_orm(string_value = "cpp")]
    Cpp,
    #[sea_orm(string_value = "rust")]
    Rust,
    #[sea_orm(string_value = "python3")]
    Python3,
}

impl Language {
    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Cpp => "cpp.txt",
            Language::Rust => "rust.txt",
            Language::Python3 => "python3.txt",
        }
    }
}
