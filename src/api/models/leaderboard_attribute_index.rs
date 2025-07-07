use std::collections::HashMap;

use serde::Serialize;

use crate::attribute_index::{AttributeIndex, AttributeKind};

#[derive(Serialize)]
pub struct LeaderboardAttributeIndex {
    pub common_global_attributes: HashMap<String, AttributeData>,
    pub common_turn_attributes: HashMap<String, AttributeData>,
    pub player_global_attributes: HashMap<String, AttributeData>,
    pub player_turn_attributes: HashMap<String, AttributeData>,
}

#[derive(Serialize)]
pub struct AttributeData {
    pub kind: Kind,
}

#[derive(Serialize)]
pub enum Kind {
    Integer,
    Float,
    String,
}

impl From<AttributeIndex> for LeaderboardAttributeIndex {
    fn from(value: AttributeIndex) -> Self {
        LeaderboardAttributeIndex {
            common_global_attributes: convert_map(value.common_global_attributes),
            common_turn_attributes: convert_map(value.common_turn_attributes),
            player_global_attributes: convert_map(value.player_global_attributes),
            player_turn_attributes: convert_map(value.player_turn_attributes),
        }
    }
}

fn convert_map(m: HashMap<String, AttributeKind>) -> HashMap<String, AttributeData> {
    m.into_iter()
        .map(|(k, v)| (k, AttributeData { kind: v.into() }))
        .collect()
}

impl From<AttributeKind> for Kind {
    fn from(value: AttributeKind) -> Self {
        match value {
            AttributeKind::Integer => Kind::Integer,
            AttributeKind::Float => Kind::Float,
            AttributeKind::String => Kind::String,
        }
    }
}
