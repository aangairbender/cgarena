use std::collections::HashMap;

use crate::domain::{Match, MatchAttributeValue};

#[derive(Default, Clone)]
pub struct AttributeIndex {
    pub common_global_attributes: HashMap<String, AttributeKind>,
    pub common_turn_attributes: HashMap<String, AttributeKind>,
    pub player_global_attributes: HashMap<String, AttributeKind>,
    pub player_turn_attributes: HashMap<String, AttributeKind>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum AttributeKind {
    #[default]
    Integer,
    Float,
    String,
}

impl AttributeKind {
    pub fn adjust(&mut self, value: &MatchAttributeValue) {
        let can_be_integer = value.integer_value().is_some();
        let can_be_float = value.float_value().is_some();

        let mut cur = *self;
        if cur == AttributeKind::Integer && !can_be_integer {
            cur = AttributeKind::Float;
        }
        if cur == AttributeKind::Float && !can_be_float {
            cur = AttributeKind::String;
        }
        *self = cur;
    }
}

impl AttributeIndex {
    pub fn process(&mut self, m: &Match) {
        for attr in &m.attributes {
            let index = match (attr.bot_id.is_some(), attr.turn.is_some()) {
                (true, true) => &mut self.player_turn_attributes,
                (true, false) => &mut self.player_global_attributes,
                (false, true) => &mut self.common_turn_attributes,
                (false, false) => &mut self.common_global_attributes,
            };

            index
                .entry(attr.name.clone())
                .or_default()
                .adjust(&attr.value);
        }
    }

    pub fn reset(&mut self) {
        self.common_global_attributes.clear();
        self.common_turn_attributes.clear();
        self.player_global_attributes.clear();
        self.player_turn_attributes.clear();
    }
}
