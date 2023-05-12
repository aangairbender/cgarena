use crate::models::{Bot, Match};

pub struct DB {
    bots: Vec<Bot>,
    matches: Vec<Match>,
}

impl DB {
    pub fn open() -> Self {
        let bots_file = std::env::current_dir()
            .unwrap()
            .join("bots")
            .join("bots.db");
        let bots = if bots_file.exists() {
            let bots_json = std::fs::read_to_string(bots_file)
                .expect("Bots file should be readable");
            serde_json::from_str(&bots_json)
                .expect("Bots should be stored in a valid JSON format")
        } else {
            Vec::new()
        };


        let matches_file = std::env::current_dir()
            .unwrap()
            .join("matches")
            .join("matches.db");
        let matches = if matches_file.exists() {
            let matches_json = std::fs::read_to_string(matches_file)
                .expect("Matches file should be readable");
            serde_json::from_str(&matches_json)
                .expect("Matches should be stored in a valid JSON format")
        } else {
            Vec::new()
        };

        Self { bots, matches }
    }

    pub fn insert_bot(&mut self, bot: Bot) {
        self.bots.push(bot);
        self.save();
    }

    pub fn delete_bot(&mut self, name: &str) {
        self.bots.retain(|b| b.name != name);
        self.save();
    }

    pub fn fetch_bots(&self) -> Vec<Bot> {
        self.bots.clone()
    }

    pub fn insert_match(&mut self, m: Match) {
        self.matches.push(m);
        self.save();
    }

    fn save(&self) {
        let bots_json = serde_json::to_string(&self.bots)
            .expect("Bots should be JSON serializable");
        let bots_file = std::env::current_dir()
            .unwrap()
            .join("bots")
            .join("bots.db");
        std::fs::write(bots_file, &bots_json)
            .expect("Should be possible to save bots");

        let matches_json = serde_json::to_string(&self.matches)
            .expect("Matches should be JSON serializable");
        let matches_file = std::env::current_dir()
            .unwrap()
            .join("matches")
            .join("matches.db");
        std::fs::write(matches_file, &matches_json)
            .expect("Should be possible to save matches");
    }
}
