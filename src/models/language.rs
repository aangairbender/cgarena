use std::{fmt::Display, str::FromStr};

use clap::ValueEnum;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, ValueEnum)]
pub enum Language {
    Cpp,
    Rust,
    Python3
}

impl Language {
    pub fn from_file_extension(s: &str) -> Option<Self> {
        match s {
            "cpp" => Some(Language::Cpp),
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python3),
            _ => None
        }
    }

    pub fn to_file_extension(&self) -> String {
        match self {
            Language::Cpp => "cpp",
            Language::Rust => "rs",
            Language::Python3 => "py",
        }.to_string()
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Cpp => write!(f, "cpp"),
            Language::Rust => write!(f, "rust"),
            Language::Python3 => write!(f, "python3"),
        }
    }
}
