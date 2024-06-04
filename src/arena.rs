use std::path::PathBuf;

use chrono::Utc;
use tokio::sync::{mpsc, oneshot};

use crate::config::Config;
use crate::model::*;

use super::db::{DBError, Database};

#[derive(thiserror::Error, Debug)]
pub enum ArenaError {
    #[error("Already exists")]
    AlreadyExists,
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

pub type ArenaResult<T> = Result<T, ArenaError>;

enum ArenaMessage {
    AddBot {
        name: String,
        source_code: String,
        language: String,
        respond_to: oneshot::Sender<ArenaResult<()>>,
    },
    RemoveBot {
        name: String,
        respond_to: oneshot::Sender<ArenaResult<()>>,
    },
    RenameBot {
        old_name: String,
        new_name: String,
        respond_to: oneshot::Sender<ArenaResult<()>>,
    },
}

struct ArenaActor {
    path: PathBuf,
    config: Config,
    receiver: mpsc::Receiver<ArenaMessage>,
    db: Database,
}

impl ArenaActor {
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&mut self, msg: ArenaMessage) {
        match msg {
            ArenaMessage::AddBot {
                name,
                source_code,
                language,
                respond_to,
            } => {
                let bot = Bot {
                    id: 0,
                    name,
                    source_code,
                    language,
                    status: BotStatus::Pending,
                    rating: Rating::default(),
                    created_at: Utc::now(),
                };
                let res = self.db.add_bot(bot).await;
                let _ = respond_to.send(res.map_err(|e| e.into()));
            }
            ArenaMessage::RemoveBot { name, respond_to } => {
                let res = self.db.remove_bot(name).await;
                let _ = respond_to.send(res.map_err(|e| e.into()));
            }
            ArenaMessage::RenameBot {
                old_name,
                new_name,
                respond_to,
            } => {
                let res = self.db.rename_bot(old_name, new_name).await;
                let _ = respond_to.send(res.map_err(|e| e.into()));
            }
        }
    }
}

#[derive(Clone)]
pub struct Arena {
    sender: mpsc::Sender<ArenaMessage>,
}

impl Arena {
    pub async fn new(path: PathBuf, config: Config, db: Database) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut actor = ArenaActor {
            path,
            config,
            receiver,
            db,
        };
        tokio::spawn(async move { actor.run().await });
        Self { sender }
    }

    pub async fn add_bot(
        &self,
        name: String,
        source_code: String,
        language: String,
    ) -> ArenaResult<()> {
        let (send, recv) = oneshot::channel();
        let msg = ArenaMessage::AddBot {
            name,
            source_code,
            language,
            respond_to: send,
        };

        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check for the
        // same failure twice.
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn remove_bot(&self, name: String) -> ArenaResult<()> {
        let (send, recv) = oneshot::channel();
        let msg = ArenaMessage::RemoveBot {
            name,
            respond_to: send,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn rename_bot(&self, old_name: String, new_name: String) -> ArenaResult<()> {
        let (send, recv) = oneshot::channel();
        let msg = ArenaMessage::RenameBot {
            old_name,
            new_name,
            respond_to: send,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}

impl From<DBError> for ArenaError {
    fn from(value: DBError) -> Self {
        match value {
            DBError::AlreadyExists => ArenaError::AlreadyExists,
            DBError::NotFound => ArenaError::NotFound,
            DBError::Unexpected(e) => ArenaError::Unexpected(e),
        }
    }
}
