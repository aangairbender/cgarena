use super::RatingSystem;

pub struct TrueSkill {

}

impl RatingSystem for TrueSkill {
    fn name() -> &'static str {
        "true_skill"
    }

    fn supports_team_vs_team() -> bool {
        true
    }

    fn supports_ffa() -> bool {
        false
    }
}