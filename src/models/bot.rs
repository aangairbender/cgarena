use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use skillratings::weng_lin::WengLinRating;

#[derive(Serialize, Deserialize, Clone)]
pub struct Bot {
    pub name: String,
    pub source_file: PathBuf,
    pub language_name: String,
    pub completed_matches: u32,
    pub raw_rating: WengLinRating,
}

impl Bot {
    pub fn new(name: String, language_name: String, source_file: PathBuf) -> Self {
        Self {
            name,
            source_file,
            language_name,
            completed_matches: 0,
            raw_rating: Default::default(),
        }
    }

    pub fn rating(&self) -> f64 {
        self.raw_rating.rating - self.raw_rating.uncertainty * 3.0
    }
}
