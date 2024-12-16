use crate::db::Database;
use crate::domain::{Match, Participant};
use crate::ranking::Ranking;
use crate::worker::PlayMatchOutput;
use itertools::Itertools;
use tokio::sync::mpsc::Receiver;

pub async fn run(mut rx: Receiver<PlayMatchOutput>, db: Database, ranking: Ranking) {
    while let Some(match_output) = rx.recv().await {
        let participants = match_output
            .bot_ids
            .into_iter()
            .zip_eq(match_output.result.ranks)
            .zip_eq(match_output.result.errors)
            .map(|((bot_id, rank), error)| Participant {
                bot_id,
                rank,
                error: error == 1,
            })
            .collect();

        let r#match = Match::new(match_output.seed, participants);
        ranking.update_rating(&r#match).await;
        db.create_match(r#match).await;
    }
}
