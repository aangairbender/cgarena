use crate::domain::BotId;

pub struct MatchAttribute {
    pub name: String,
    pub bot_id: Option<BotId>,
    pub turn: Option<u16>,
    pub value: MatchAttributeValue,
}

pub enum MatchAttributeValue {
    Integer(i64),
    Float(f64),
    String(String),
}

impl MatchAttributeValue {
    pub fn integer_value(&self) -> Option<i64> {
        if let Self::Integer(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn float_value(&self) -> Option<f64> {
        if let Self::Float(v) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn string_value(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v.as_str())
        } else {
            None
        }
    }
}

impl From<String> for MatchAttributeValue {
    fn from(value: String) -> Self {
        if let Ok(v) = value.parse::<i64>() {
            MatchAttributeValue::Integer(v)
        } else if let Ok(v) = value.parse::<f64>() {
            MatchAttributeValue::Float(v)
        } else {
            MatchAttributeValue::String(value)
        }
    }
}
