use async_trait::async_trait;
use uuid::Uuid;

use super::{Language, Match};

pub struct Worker {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
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