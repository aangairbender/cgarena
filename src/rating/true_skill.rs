use super::RatingSystem;

pub struct TrueSkill {

}

impl RatingSystem for TrueSkill {
    fn name() -> &'static str {
        "true_skill"
    }
}