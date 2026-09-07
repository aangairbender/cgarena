use std::{fmt, str::FromStr};

use anyhow::bail;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotRole {
    Candidate,
    Benchmark,
    ArchivedBenchmark,
}

impl Default for BotRole {
    fn default() -> Self {
        Self::Candidate
    }
}

impl BotRole {
    pub fn is_active(self) -> bool {
        !matches!(self, Self::ArchivedBenchmark)
    }
}

impl fmt::Display for BotRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Candidate => "candidate",
            Self::Benchmark => "benchmark",
            Self::ArchivedBenchmark => "archived_benchmark",
        })
    }
}

impl FromStr for BotRole {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "candidate" => Ok(Self::Candidate),
            "benchmark" => Ok(Self::Benchmark),
            "archived_benchmark" => Ok(Self::ArchivedBenchmark),
            _ => bail!("unknown bot role {value}"),
        }
    }
}
