use crate::arena::{
    ArenaCommand, CreateBotCommand, CreateBotResult, CreateLeaderboardCommand, DeleteBotCommand,
    DeleteLeaderboardCommand, FetchStatusCommand, FetchStatusResult, LeaderboardOverview,
    RenameBotCommand, RenameBotResult, RenameLeaderboardCommand, RenameLeaderboardResult,
};
use crate::domain::{
    BotId, BotName, Language, LeaderboardId, LeaderboardName, MatchFilter, SourceCode,
};
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
        })
        .await
    }

    pub async fn rename_bot(&self, id: BotId, new_name: BotName) -> RenameBotResult {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::RenameBot(RenameBotCommand {
                id,
                new_name,
                response: tx,
            })
        })
        .await
    }

    pub async fn delete_bot(&self, id: BotId) {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::DeleteBot(DeleteBotCommand { id, response: tx })
        })
        .await
    }

    pub async fn fetch_status(&self) -> FetchStatusResult {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::FetchStatus(FetchStatusCommand { response: tx })
        })
        .await
    }

    pub async fn create_leaderboard(
        &self,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> LeaderboardOverview {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::CreateLeaderboard(CreateLeaderboardCommand {
                name,
                filter,
                response: tx,
            })
        })
        .await
    }

    pub async fn rename_leaderboard(
        &self,
        id: LeaderboardId,
        new_name: LeaderboardName,
    ) -> RenameLeaderboardResult {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::RenameLeaderboard(RenameLeaderboardCommand {
                id,
                new_name,
                response: tx,
            })
        })
        .await
    }

    pub async fn delete_leaderboard(&self, id: LeaderboardId) {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::DeleteLeaderboard(DeleteLeaderboardCommand { id, response: tx })
        })
        .await
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
