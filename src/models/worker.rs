use async_trait::async_trait;

use super::{Language, Match};

pub struct Worker {
    pub name: String,
}

#[async_trait]
trait WorkerTrait {
    async fn thread_cnt(&self) -> u32;
    async fn supports(&self, language: &Language) -> bool;
    async fn queue_match(&self, m: &Match);
}

struct EmbeddedWorker {}

struct StandaloneWorker {
    
}

impl StandaloneWorker {

}