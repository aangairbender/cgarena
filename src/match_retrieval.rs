//! Deep read interface for persisted matches.
//!
//! Callers choose an operation (`page`, `leaderboard_matches`, or `chart_input`)
//! and receive domain data for that use case. Filter planning, bounded SQLite
//! scans, ordering, lookahead, hydration, and participant-name enrichment stay
//! inside this module.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::{bail, Context};
use itertools::Itertools;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use tracing::warn;

use crate::domain::{
    BotId, BotName, Match, MatchAttribute, MatchAttributeValue, MatchFilter, MatchId, Participant,
};

const CANDIDATE_BATCH_SIZE: usize = 256;
const MATCHES_PER_CHART: usize = 1000;

/// The match retrieval interface shared by HTTP, leaderboards, and charting.
///
/// Results are newest first. `page` returns fully hydrated non-turn attributes
/// and participant names; leaderboard and chart operations fetch only the data
/// their callers need.
#[derive(Clone)]
pub(crate) struct MatchRetrieval {
    pool: SqlitePool,
}

pub(crate) struct MatchPageRequest {
    pub filter: MatchFilter,
    pub including_bots: Vec<BotId>,
    pub offset: usize,
    pub limit: usize,
}

pub(crate) struct MatchPage {
    pub matches: Vec<MatchOverview>,
    pub has_more: bool,
}

pub(crate) struct MatchOverview {
    pub id: MatchId,
    pub participants: Vec<ParticipantOverview>,
    pub seed: i64,
    pub attributes: Vec<MatchAttribute>,
}

pub(crate) struct ParticipantOverview {
    pub bot_id: BotId,
    pub bot_name: BotName,
    pub rank: u8,
    pub index: usize,
    pub error: bool,
}

pub(crate) struct ChartInput {
    pub attributes: Vec<MatchAttribute>,
    pub total_matches: u64,
}

impl MatchRetrieval {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetches one enriched page and uses one extra match for page lookahead.
    pub(crate) async fn page(&self, request: MatchPageRequest) -> anyhow::Result<MatchPage> {
        let MatchPageRequest {
            filter,
            including_bots,
            offset,
            limit,
        } = request;
        let including_bots = included_bot_ids(including_bots);
        let requested_matches = limit.saturating_add(1);
        let mut matches = self
            .matching_matches(&filter, &including_bots, offset, Some(requested_matches))
            .await?;

        let has_more = matches.len() > limit;
        matches.truncate(limit);
        self.hydrate_non_turn_attributes(&mut matches).await?;
        let bot_names = self.fetch_bot_names(&matches).await?;
        let matches = matches
            .into_iter()
            .map(|item| match_overview(item, &bot_names))
            .try_collect()?;

        Ok(MatchPage { matches, has_more })
    }

    /// Fetches every matching match once for a complete leaderboard rebuild.
    pub(crate) async fn leaderboard_matches(
        &self,
        filter: &MatchFilter,
    ) -> anyhow::Result<Vec<Match>> {
        self.matching_matches(filter, &HashSet::new(), 0, None)
            .await
    }

    /// Fetches the latest chart window and its requested per-turn attributes.
    pub(crate) async fn chart_input(
        &self,
        filter: &MatchFilter,
        attribute_name: &str,
    ) -> anyhow::Result<ChartInput> {
        let matches = self
            .matching_matches(filter, &HashSet::new(), 0, Some(MATCHES_PER_CHART))
            .await?;
        let match_ids = matches.into_iter().map(|item| item.id).collect_vec();
        let attributes = self
            .fetch_turn_attributes(&match_ids, attribute_name)
            .await?;

        Ok(ChartInput {
            attributes,
            total_matches: match_ids.len() as u64,
        })
    }

    async fn matching_matches(
        &self,
        filter: &MatchFilter,
        including_bots: &HashSet<i64>,
        offset: usize,
        max_matches: Option<usize>,
    ) -> anyhow::Result<Vec<Match>> {
        if filter.needed_attributes().is_empty() {
            let empty_match = Match::new(0, vec![], vec![], None);
            if !filter.matches(&empty_match) {
                return Ok(vec![]);
            }
            if let Some(max_matches) = max_matches {
                return self
                    .fetch_unfiltered_matches(including_bots, offset, max_matches)
                    .await;
            }
        }

        self.collect_matching_matches(filter, including_bots, offset, max_matches)
            .await
    }

    async fn fetch_unfiltered_matches(
        &self,
        including_bots: &HashSet<i64>,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<Match>> {
        let candidates = self
            .fetch_match_candidates(including_bots, None, offset, limit)
            .await?;
        let candidate_ids = candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect_vec();
        let mut participations = self.fetch_participations(&candidate_ids).await?;

        Ok(candidates
            .into_iter()
            .filter_map(|candidate| {
                let candidate_id = candidate.id;
                Match::try_from((
                    candidate,
                    participations.remove(&candidate_id).unwrap_or_default(),
                    vec![],
                ))
                .inspect_err(|error| {
                    warn!(
                        "Invalid db data (match {}): {}. Skipping.",
                        candidate_id, error
                    )
                })
                .ok()
            })
            .collect())
    }

    async fn collect_matching_matches(
        &self,
        filter: &MatchFilter,
        including_bots: &HashSet<i64>,
        offset: usize,
        max_matches: Option<usize>,
    ) -> anyhow::Result<Vec<Match>> {
        let needed_attributes = filter
            .needed_attributes()
            .into_iter()
            .map(|attr| (attr.name, attr.bot_id.map(i64::from), attr.turn))
            .unique()
            .collect_vec();
        let mut before_id = None;
        let mut matching_before_page = 0;
        let mut matches = Vec::new();

        loop {
            let candidates = self
                .fetch_match_candidates(including_bots, before_id, 0, CANDIDATE_BATCH_SIZE)
                .await?;
            if candidates.is_empty() {
                break;
            }
            before_id = candidates.last().map(|candidate| candidate.id);

            let candidate_ids = candidates
                .iter()
                .map(|candidate| candidate.id)
                .collect_vec();
            let candidate_count = candidate_ids.len();
            let mut participations = self.fetch_participations(&candidate_ids).await?;
            let mut attributes = self
                .fetch_filter_attributes(&candidate_ids, &needed_attributes)
                .await?;

            for candidate in candidates {
                let candidate_id = candidate.id;
                let item = (
                    candidate,
                    participations.remove(&candidate_id).unwrap_or_default(),
                    attributes.remove(&candidate_id).unwrap_or_default(),
                );
                let candidate = match Match::try_from(item) {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        warn!(
                            "Invalid db data (match {}): {}. Skipping.",
                            candidate_id, error
                        );
                        continue;
                    }
                };
                if !filter.matches(&candidate) {
                    continue;
                }
                if matching_before_page < offset {
                    matching_before_page += 1;
                    continue;
                }

                matches.push(candidate);
                if max_matches.is_some_and(|maximum| matches.len() >= maximum) {
                    return Ok(matches);
                }
            }

            if candidate_count < CANDIDATE_BATCH_SIZE {
                break;
            }
        }

        Ok(matches)
    }

    async fn fetch_match_candidates(
        &self,
        including_bots: &HashSet<i64>,
        before_id: Option<i64>,
        offset: usize,
        limit: usize,
    ) -> anyhow::Result<Vec<MatchesRow>> {
        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT m.id, m.seed, m.participant_cnt, m.replay_path FROM matches m",
        );
        let mut has_where = false;

        for bot_id in including_bots.iter().sorted() {
            query.push(if has_where {
                " AND EXISTS ("
            } else {
                " WHERE EXISTS ("
            });
            query.push(
                "SELECT 1 FROM participations p \
                 WHERE p.match_id = m.id AND p.bot_id = ",
            );
            query.push_bind(bot_id);
            query.push(")");
            has_where = true;
        }

        if let Some(before_id) = before_id {
            query.push(if has_where {
                " AND m.id < "
            } else {
                " WHERE m.id < "
            });
            query.push_bind(before_id);
        }

        query.push(" ORDER BY m.id DESC LIMIT ");
        query.push_bind(i64::try_from(limit)?);
        if offset != 0 {
            query.push(" OFFSET ");
            query.push_bind(i64::try_from(offset)?);
        }
        Ok(query.build_query_as().fetch_all(&self.pool).await?)
    }

    async fn fetch_participations(
        &self,
        match_ids: &[i64],
    ) -> anyhow::Result<HashMap<i64, Vec<ParticipationsRow>>> {
        if match_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT match_id, bot_id, `index`, rank, error FROM participations WHERE match_id IN (",
        );
        let mut separated = query.separated(", ");
        for match_id in match_ids {
            separated.push_bind(match_id);
        }
        separated.push_unseparated(")");

        Ok(query
            .build_query_as::<ParticipationsRow>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| (row.match_id, row))
            .into_group_map())
    }

    async fn fetch_filter_attributes(
        &self,
        match_ids: &[i64],
        needed_attributes: &[(String, Option<i64>, Option<u16>)],
    ) -> anyhow::Result<HashMap<i64, Vec<MatchAttributesJoinedRow>>> {
        if match_ids.is_empty() || needed_attributes.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = joined_attributes_query();
        query.push(" WHERE ma.match_id IN (");
        let mut separated = query.separated(", ");
        for match_id in match_ids {
            separated.push_bind(match_id);
        }
        separated.push_unseparated(") AND (");

        for (index, (name, bot_id, turn)) in needed_attributes.iter().enumerate() {
            if index != 0 {
                query.push(" OR ");
            }
            query.push("(n.name = ");
            query.push_bind(name);
            match bot_id {
                Some(bot_id) => {
                    query.push(" AND ma.bot_id = ");
                    query.push_bind(bot_id);
                }
                None => {
                    query.push(" AND ma.bot_id IS NULL");
                }
            }
            match turn {
                Some(turn) => {
                    query.push(" AND ma.turn = ");
                    query.push_bind(turn);
                }
                None => {
                    query.push(" AND ma.turn IS NULL");
                }
            }
            query.push(")");
        }
        query.push(")");

        Ok(query
            .build_query_as::<MatchAttributesJoinedRow>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| (row.match_id, row))
            .into_group_map())
    }

    async fn hydrate_non_turn_attributes(&self, matches: &mut [Match]) -> anyhow::Result<()> {
        if matches.is_empty() {
            return Ok(());
        }

        let mut query = joined_attributes_query();
        query.push(" WHERE ma.turn IS NULL AND ma.match_id IN (");
        let mut separated = query.separated(", ");
        for item in matches.iter() {
            separated.push_bind(i64::from(item.id));
        }
        separated.push_unseparated(")");

        let mut attributes = query
            .build_query_as::<MatchAttributesJoinedRow>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| (row.match_id, row))
            .into_group_map();

        for item in matches {
            item.attributes = attributes
                .remove(&i64::from(item.id))
                .unwrap_or_default()
                .into_iter()
                .filter_map(|attribute| {
                    let match_id = attribute.match_id;
                    MatchAttribute::try_from(attribute)
                        .inspect_err(|error| {
                            warn!(
                                "Invalid db data (match attribute of match {}): {}. Skipping.",
                                match_id, error
                            )
                        })
                        .ok()
                })
                .collect();
        }
        Ok(())
    }

    async fn fetch_bot_names(&self, matches: &[Match]) -> anyhow::Result<HashMap<BotId, BotName>> {
        let bot_ids = matches
            .iter()
            .flat_map(|item| item.participants.iter())
            .map(|participant| i64::from(participant.bot_id))
            .unique()
            .collect_vec();
        if bot_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut query = QueryBuilder::<Sqlite>::new("SELECT id, name FROM bots WHERE id IN (");
        let mut separated = query.separated(", ");
        for bot_id in bot_ids {
            separated.push_bind(bot_id);
        }
        separated.push_unseparated(")");

        query
            .build_query_as::<BotNameRow>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| {
                let id: BotId = row.id.into();
                let name = row
                    .name
                    .try_into()
                    .with_context(|| format!("Invalid name for bot {id}"))?;
                Ok((id, name))
            })
            .collect()
    }

    async fn fetch_turn_attributes(
        &self,
        match_ids: &[MatchId],
        attribute_name: &str,
    ) -> anyhow::Result<Vec<MatchAttribute>> {
        if match_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT \
             n.name AS name, \
             ma.match_id AS match_id, \
             ma.bot_id AS bot_id, \
             ma.turn AS turn, \
             ma.value_int AS value_int, \
             ma.value_float AS value_float, \
             NULL AS value_string \
             FROM match_attributes ma \
             INNER JOIN match_attribute_names n ON n.id = ma.name_id \
             WHERE n.name = ",
        );
        query.push_bind(attribute_name);
        query.push(" AND ma.bot_id IS NOT NULL AND ma.turn IS NOT NULL AND ma.match_id IN (");
        let mut separated = query.separated(", ");
        for match_id in match_ids {
            separated.push_bind(i64::from(*match_id));
        }
        separated.push_unseparated(")");

        Ok(query
            .build_query_as::<MatchAttributesJoinedRow>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .filter_map(|row| MatchAttribute::try_from(row).ok())
            .collect())
    }
}

fn included_bot_ids(bot_ids: Vec<BotId>) -> HashSet<i64> {
    bot_ids.into_iter().map(i64::from).collect()
}

fn match_overview(
    item: Match,
    bot_names: &HashMap<BotId, BotName>,
) -> anyhow::Result<MatchOverview> {
    let participants = item
        .participants
        .into_iter()
        .enumerate()
        .map(
            |(index, participant)| -> anyhow::Result<ParticipantOverview> {
                Ok(ParticipantOverview {
                    bot_id: participant.bot_id,
                    bot_name: bot_names
                        .get(&participant.bot_id)
                        .cloned()
                        .with_context(|| format!("Bot {} is missing", participant.bot_id))?,
                    rank: participant.rank,
                    index,
                    error: participant.error,
                })
            },
        )
        .try_collect()?;

    Ok(MatchOverview {
        id: item.id,
        participants,
        seed: item.seed,
        attributes: item.attributes,
    })
}

fn joined_attributes_query<'args>() -> QueryBuilder<'args, Sqlite> {
    QueryBuilder::new(
        "SELECT \
         n.name AS name, \
         ma.match_id AS match_id, \
         ma.bot_id AS bot_id, \
         ma.turn AS turn, \
         ma.value_int AS value_int, \
         ma.value_float AS value_float, \
         v.value AS value_string \
         FROM match_attributes ma \
         INNER JOIN match_attribute_names n ON n.id = ma.name_id \
         LEFT JOIN match_attribute_string_values v ON v.id = ma.value_string_id",
    )
}

#[derive(sqlx::FromRow)]
struct BotNameRow {
    id: i64,
    name: String,
}

#[derive(sqlx::FromRow)]
struct MatchesRow {
    id: i64,
    seed: i64,
    participant_cnt: u8,
    replay_path: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ParticipationsRow {
    match_id: i64,
    bot_id: i64,
    index: u8,
    rank: u8,
    error: bool,
}

#[derive(sqlx::FromRow)]
struct MatchAttributesJoinedRow {
    name: String,
    match_id: i64,
    bot_id: Option<i64>,
    turn: Option<u16>,
    value_int: Option<i64>,
    value_float: Option<f64>,
    value_string: Option<String>,
}

impl From<ParticipationsRow> for Participant {
    fn from(row: ParticipationsRow) -> Self {
        Participant {
            bot_id: row.bot_id.into(),
            rank: row.rank,
            error: row.error,
        }
    }
}

impl TryFrom<MatchAttributesJoinedRow> for MatchAttribute {
    type Error = anyhow::Error;

    fn try_from(row: MatchAttributesJoinedRow) -> Result<Self, Self::Error> {
        Ok(MatchAttribute {
            name: row.name,
            bot_id: row.bot_id.map(Into::into),
            turn: row.turn,
            value: match (row.value_int, row.value_float, row.value_string) {
                (Some(value), None, None) => MatchAttributeValue::Integer(value),
                (None, Some(value), None) => MatchAttributeValue::Float(value),
                (None, None, Some(value)) => MatchAttributeValue::String(value),
                _ => bail!("Ambiguous attribute value type. match id {}", row.match_id),
            },
        })
    }
}

impl
    TryFrom<(
        MatchesRow,
        Vec<ParticipationsRow>,
        Vec<MatchAttributesJoinedRow>,
    )> for Match
{
    type Error = anyhow::Error;

    fn try_from(
        (item, mut participations, attributes): (
            MatchesRow,
            Vec<ParticipationsRow>,
            Vec<MatchAttributesJoinedRow>,
        ),
    ) -> Result<Self, Self::Error> {
        if item.participant_cnt as usize != participations.len() {
            bail!("participant count mismatch");
        }
        participations.sort_by_key(|participation| participation.index);
        for (index, participation) in participations.iter().enumerate() {
            if index != participation.index as usize {
                bail!("Some participation index is missing");
            }
        }

        Ok(Match {
            id: item.id.into(),
            seed: item.seed,
            replay_path: item.replay_path.map(PathBuf::from),
            participants: participations.into_iter().map(Into::into).collect(),
            attributes: attributes
                .into_iter()
                .filter_map(|attribute| {
                    let match_id = attribute.match_id;
                    MatchAttribute::try_from(attribute)
                        .inspect_err(|error| {
                            warn!(
                                "Invalid db data (match attribute of match {}): {}. Skipping.",
                                match_id, error
                            )
                        })
                        .ok()
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn retrieval() -> MatchRetrieval {
        let pool = db::in_memory().await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        MatchRetrieval::new(pool)
    }

    async fn insert_test_bot(retrieval: &MatchRetrieval, name: &str) -> BotId {
        sqlx::query(
            "INSERT INTO bots (name, source_code, language, created_at) VALUES (?, '', 'rust', 0)",
        )
        .bind(name)
        .execute(&retrieval.pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into()
    }

    async fn insert_test_match(
        retrieval: &MatchRetrieval,
        seed: i64,
        bot_ids: &[BotId],
        attributes: Vec<MatchAttribute>,
    ) -> MatchId {
        let participants = bot_ids
            .iter()
            .enumerate()
            .map(|(index, bot_id)| Participant {
                bot_id: *bot_id,
                rank: index as u8,
                error: false,
            })
            .collect();
        db::create_match(
            &retrieval.pool,
            &Match::new(seed, participants, attributes, None),
        )
        .await
        .unwrap()
    }

    fn request(
        filter: MatchFilter,
        including_bots: Vec<BotId>,
        offset: usize,
        limit: usize,
    ) -> MatchPageRequest {
        MatchPageRequest {
            filter,
            including_bots,
            offset,
            limit,
        }
    }

    fn ids(page: &MatchPage) -> Vec<MatchId> {
        page.matches.iter().map(|item| item.id).collect()
    }

    #[tokio::test]
    async fn paginates_empty_exact_and_past_end_in_newest_order() {
        let retrieval = retrieval().await;
        let bot = insert_test_bot(&retrieval, "bot").await;
        let filter = MatchFilter::accept_all();

        let empty = retrieval
            .page(request(filter.clone(), vec![], 0, 2))
            .await
            .unwrap();
        assert!(empty.matches.is_empty());
        assert!(!empty.has_more);

        let oldest = insert_test_match(&retrieval, 1, &[bot], vec![]).await;
        let middle = insert_test_match(&retrieval, 2, &[bot], vec![]).await;
        let newest = insert_test_match(&retrieval, 3, &[bot], vec![]).await;

        let first = retrieval
            .page(request(filter.clone(), vec![], 0, 2))
            .await
            .unwrap();
        assert_eq!(ids(&first), vec![newest, middle]);
        assert!(first.has_more);
        assert_eq!(first.matches[0].participants[0].bot_name.to_string(), "bot");

        let exact_last = retrieval
            .page(request(filter.clone(), vec![], 2, 1))
            .await
            .unwrap();
        assert_eq!(ids(&exact_last), vec![oldest]);
        assert!(!exact_last.has_more);

        let exact_end = retrieval
            .page(request(filter.clone(), vec![], 3, 1))
            .await
            .unwrap();
        assert!(exact_end.matches.is_empty());
        assert!(!exact_end.has_more);

        let past_end = retrieval
            .page(request(filter, vec![], 100, 10))
            .await
            .unwrap();
        assert!(past_end.matches.is_empty());
        assert!(!past_end.has_more);
    }

    #[tokio::test]
    async fn constant_false_filter_returns_no_candidates() {
        let retrieval = retrieval().await;
        let bot = insert_test_bot(&retrieval, "bot").await;
        insert_test_match(&retrieval, 1, &[bot], vec![]).await;
        let filter: MatchFilter = "1 == 2".parse().unwrap();

        let page = retrieval
            .page(request(filter, vec![], 0, 10))
            .await
            .unwrap();

        assert!(page.matches.is_empty());
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn requires_every_included_bot() {
        let retrieval = retrieval().await;
        let bot_1 = insert_test_bot(&retrieval, "bot 1").await;
        let bot_2 = insert_test_bot(&retrieval, "bot 2").await;
        let bot_3 = insert_test_bot(&retrieval, "bot 3").await;
        let filter = MatchFilter::accept_all();

        let both_old = insert_test_match(&retrieval, 1, &[bot_1, bot_2], vec![]).await;
        insert_test_match(&retrieval, 2, &[bot_1, bot_3], vec![]).await;
        let both_new = insert_test_match(&retrieval, 3, &[bot_1, bot_2, bot_3], vec![]).await;
        insert_test_match(&retrieval, 4, &[bot_2, bot_3], vec![]).await;

        let page = retrieval
            .page(request(filter, vec![bot_1, bot_2, bot_1], 0, 10))
            .await
            .unwrap();
        assert_eq!(ids(&page), vec![both_new, both_old]);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn fills_filtered_pages_across_candidate_batches() {
        let retrieval = retrieval().await;
        let bot = insert_test_bot(&retrieval, "bot").await;
        let score = |value| MatchAttribute {
            name: "score".to_string(),
            bot_id: Some(bot),
            turn: None,
            value: MatchAttributeValue::Integer(value),
        };
        let label = |value: &str| MatchAttribute {
            name: "label".to_string(),
            bot_id: None,
            turn: None,
            value: MatchAttributeValue::String(value.to_string()),
        };

        let older_match =
            insert_test_match(&retrieval, 1, &[bot], vec![score(10), label("older")]).await;
        let newer_match =
            insert_test_match(&retrieval, 2, &[bot], vec![score(20), label("newer")]).await;
        for seed in 3..259 {
            insert_test_match(&retrieval, seed, &[bot], vec![]).await;
        }

        let filter: MatchFilter = format!("bot({bot}).score >= 10").parse().unwrap();
        let first = retrieval
            .page(request(filter.clone(), vec![], 0, 1))
            .await
            .unwrap();
        assert_eq!(ids(&first), vec![newer_match]);
        assert!(first.has_more);
        assert!(first.matches[0].attributes.iter().any(|attribute| {
            attribute.name == "label"
                && matches!(
                    &attribute.value,
                    MatchAttributeValue::String(value) if value == "newer"
                )
        }));

        let second = retrieval
            .page(request(filter.clone(), vec![], 1, 1))
            .await
            .unwrap();
        assert_eq!(ids(&second), vec![older_match]);
        assert!(!second.has_more);

        let end = retrieval.page(request(filter, vec![], 2, 1)).await.unwrap();
        assert!(end.matches.is_empty());
        assert!(!end.has_more);
    }

    #[tokio::test]
    async fn fetches_all_leaderboard_matches_without_duplicates() {
        let retrieval = retrieval().await;
        let bot = insert_test_bot(&retrieval, "bot").await;
        let mut inserted = Vec::new();
        for seed in 0..300 {
            inserted.push(insert_test_match(&retrieval, seed, &[bot], vec![]).await);
        }

        let matches = retrieval
            .leaderboard_matches(&MatchFilter::accept_all())
            .await
            .unwrap();
        let ids = matches.iter().map(|item| item.id).collect_vec();
        let expected = inserted.into_iter().rev().collect_vec();

        assert_eq!(ids, expected);
    }

    #[tokio::test]
    async fn supplies_chart_attributes_for_matching_matches() {
        let retrieval = retrieval().await;
        let bot = insert_test_bot(&retrieval, "bot").await;
        let match_score = |value| MatchAttribute {
            name: "score".to_string(),
            bot_id: None,
            turn: None,
            value: MatchAttributeValue::Integer(value),
        };
        let turn_score = |value| MatchAttribute {
            name: "score".to_string(),
            bot_id: Some(bot),
            turn: Some(1),
            value: MatchAttributeValue::Integer(value),
        };
        insert_test_match(
            &retrieval,
            1,
            &[bot],
            vec![match_score(10), turn_score(100)],
        )
        .await;
        insert_test_match(
            &retrieval,
            2,
            &[bot],
            vec![match_score(20), turn_score(200)],
        )
        .await;
        let filter: MatchFilter = "match.score == 10".parse().unwrap();

        let input = retrieval.chart_input(&filter, "score").await.unwrap();

        assert_eq!(input.total_matches, 1);
        assert_eq!(input.attributes.len(), 1);
        assert_eq!(input.attributes[0].bot_id, Some(bot));
        assert!(matches!(
            &input.attributes[0].value,
            MatchAttributeValue::Integer(100)
        ));
    }
}
