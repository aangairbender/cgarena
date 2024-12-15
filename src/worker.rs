use crate::domain::BotId;
use crate::embedded_worker::EmbeddedWorker;

pub enum Worker {
    Embedded(EmbeddedWorker),
    // Remote,
}

impl Worker {
    pub fn name(&self) -> &str {
        match self {
            Worker::Embedded(_) => "embedded",
        }
    }
    pub async fn build(&self, input: BuildInput) -> Result<BuildOutput, anyhow::Error> {
        match self {
            Worker::Embedded(w) => w.build(input).await,
        }
    }
}

#[derive(Clone)]
pub struct BuildInput {
    pub bot_id: BotId,
    pub source_code: String,
    pub language: String,
}

pub struct BuildOutput {
    pub status_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}
