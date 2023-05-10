use serde::{Serialize, Deserialize};
use skillratings::weng_lin::WengLinRating;
use tabled::Tabled;
use uuid::Uuid;

use super::Language;

#[derive(Serialize, Deserialize, Tabled, Clone)]
pub struct Bot {
    #[tabled(skip)]
    pub id: Uuid,
    pub name: String,
    #[tabled(skip)]
    pub description: String,
    #[tabled(skip)]
    pub source_code_file: String,
    pub language: Language,
    #[tabled(rename = "matches")]
    pub completed_matches: u32,
    #[tabled(display_with = "display_rating")]
    pub rating: WengLinRating,
}

impl Bot {
    pub fn new(name: String, description: String, source_code_file: String, language: Language) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            source_code_file,
            language,
            completed_matches: 0,
            rating: Default::default(),
        }
    }

    pub fn estimated_rating(&self) -> f64 {
        self.rating.rating - self.rating.uncertainty * 3.0
    }
}

fn display_rating(rating: &WengLinRating) -> String {
    format!("{}", rating.rating - rating.uncertainty * 3.0)
}
