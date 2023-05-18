mod true_skill;

pub use true_skill::*;

trait RatingSystem {
    fn name() -> &'static str;
}

trait PvPRatingSystem {
    fn pvp();
}

trait TeamVsTeamRatingSystem {
    fn team_vs_team();
}

trait MultiteamRatingSystem {
    fn multiteam();
}
