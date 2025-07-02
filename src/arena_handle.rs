use crate::arena::{
    ArenaCommand, BotMinimal, CreateBotCommand, CreateBotResult, DeleteBotCommand, FetchBotsCommand, FetchLeaderboardCommand, FetchLeaderboardResult, RenameBotCommand, RenameBotResult
};
use crate::domain::{BotId, BotName, Language, SourceCode};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct ArenaHandle {
    commands_tx: mpsc::Sender<ArenaCommand>,
}

impl ArenaHandle {
    pub fn new(commands_tx: mpsc::Sender<ArenaCommand>) -> Self {
        Self { commands_tx }
    }

    pub async fn create_bot(
        &self,
        name: BotName,
        source_code: SourceCode,
        language: Language,
    ) -> CreateBotResult {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::CreateBot(CreateBotCommand {
                name,
                source_code,
                language,
                response: tx,
            })
        }).await
    }

    pub async fn rename_bot(&self, id: BotId, new_name: BotName) -> RenameBotResult {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::RenameBot(RenameBotCommand {
                id,
                new_name,
                response: tx,
            })
        }).await
    }

    pub async fn delete_bot(&self, id: BotId) {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::DeleteBot(DeleteBotCommand { id, response: tx })
        }).await
    }

    pub async fn fetch_leaderboard(&self, id: BotId) -> Option<FetchLeaderboardResult> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::FetchLeaderboard(FetchLeaderboardCommand {
                bot_id: id,
                response: tx,
            })
        }).await
    }

    pub async fn fetch_all_bots(&self) -> Vec<BotMinimal> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::FetchBots(FetchBotsCommand { response: tx })
        }).await
    }

    async fn send_command_and_await_for_result<R, F: FnOnce(oneshot::Sender<R>) -> ArenaCommand>(
        &self,
        cmd_builder: F,
    ) -> R {
        let (tx, rx) = oneshot::channel();

        let cmd = cmd_builder(tx);

        self.commands_tx
            .send(cmd)
            .await
            .expect("Arena command tx send error");

        rx.await.expect("Arena command rx error")
    }
}
