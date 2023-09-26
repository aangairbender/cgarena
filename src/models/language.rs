use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")] 
pub enum Language {
    Cpp,
    Rust,
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
