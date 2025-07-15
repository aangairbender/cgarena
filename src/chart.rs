use std::collections::HashMap;

use itertools::Itertools;
use sqlx::SqlitePool;

use crate::{
    arena::{ChartItem, ChartOverview, ChartTurnData},
    db,
    domain::{BotId, MatchAttributeValue, MatchFilter, MatchId},
};

pub async fn visualize(
    filter: MatchFilter,
    attribute_name: String,
    pool: SqlitePool,
) -> anyhow::Result<ChartOverview> {
    let needed_attrs = filter.needed_attributes();
    let matches = db::fetch_matches_with_attrs(&pool, &needed_attrs).await?;

    let filtered_match_ids: Vec<MatchId> = matches
        .iter()
        .filter(|&m| filter.matches(m))
        .map(|m| m.id)
        .collect();

    let last_match_ids: Vec<MatchId> = filtered_match_ids.into_iter().rev().take(1000).collect();

    let data = db::fetch_turn_attributes(&pool, &last_match_ids, &attribute_name).await?;

    let mut res: HashMap<BotId, HashMap<u16, Stats>> = HashMap::new();

    for attr in data {
        if attr.name != attribute_name {
            continue;
        }
        let Some(bot_id) = attr.bot_id else {
            continue;
        };
        let Some(turn) = attr.turn else {
            continue;
        };
        let v = match attr.value {
            MatchAttributeValue::Integer(x) => x as f64,
            MatchAttributeValue::Float(x) => x,
            MatchAttributeValue::String(_) => continue,
        };

        res.entry(bot_id)
            .or_default()
            .entry(turn)
            .or_default()
            .adjust(v);
    }

    let overview = ChartOverview {
        items: res
            .into_iter()
            .map(|(bot_id, data)| ChartItem {
                bot_id,
                data: data
                    .into_iter()
                    .map(|(turn, stats)| ChartTurnData {
                        turn,
                        avg: if stats.cnt == 0 {
                            0.0
                        } else {
                            stats.sum / stats.cnt as f64
                        },
                        min: stats.min,
                        max: stats.max,
                    })
                    .sorted_by_key(|w| w.turn)
                    .collect(),
            })
            .collect(),
        total_matches: last_match_ids.len() as _,
    };

    Ok(overview)
}

#[derive(Default)]
struct Stats {
    sum: f64,
    min: f64,
    max: f64,
    cnt: u64,
}

impl Stats {
    fn adjust(&mut self, v: f64) {
        self.sum += v;
        self.min = self.min.min(v);
        self.max = self.max.max(v);
        self.cnt += 1;
    }
}
