use crate::arena_commands::{
    ArenaCommand, BotSourceCode, ChartCommand, ChartOverview, CreateBotCommand, CreateBotResult,
    CreateLeaderboardCommand, DeleteBotCommand, DeleteLeaderboardCommand, EnableMatchmakingCommand,
    FetchBotSourceCodeCommand, FetchStatusCommand, FetchStatusResult, LeaderboardOverview,
    PatchLeaderboardCommand, PatchLeaderboardResult, RenameBotCommand, RenameBotResult,
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

    pub async fn fetch_bot_source_code(&self, id: BotId) -> anyhow::Result<Option<BotSourceCode>> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::FetchBotSourceCode(FetchBotSourceCodeCommand { id, response: tx })
        })
        .await
    }

    pub async fn create_bot(
        &self,
        name: BotName,
        source_code: SourceCode,
        language: Language,
    ) -> anyhow::Result<CreateBotResult> {
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

    pub async fn rename_bot(
        &self,
        id: BotId,
        new_name: BotName,
    ) -> anyhow::Result<RenameBotResult> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::RenameBot(RenameBotCommand {
                id,
                new_name,
                response: tx,
            })
        })
        .await
    }

    pub async fn delete_bot(&self, id: BotId) -> anyhow::Result<()> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::DeleteBot(DeleteBotCommand { id, response: tx })
        })
        .await
    }

    pub async fn fetch_status(&self) -> anyhow::Result<FetchStatusResult> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::FetchStatus(FetchStatusCommand { response: tx })
        })
        .await
    }

    pub async fn create_leaderboard(
        &self,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> anyhow::Result<LeaderboardOverview> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::CreateLeaderboard(CreateLeaderboardCommand {
                name,
                filter,
                response: tx,
            })
        })
        .await
    }

    pub async fn patch_leaderboard(
        &self,
        id: LeaderboardId,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> anyhow::Result<PatchLeaderboardResult> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::PatchLeaderboard(PatchLeaderboardCommand {
                id,
                name,
                filter,
                response: tx,
            })
        })
        .await
    }

    pub async fn delete_leaderboard(&self, id: LeaderboardId) -> anyhow::Result<()> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::DeleteLeaderboard(DeleteLeaderboardCommand { id, response: tx })
        })
        .await
    }

    pub async fn chart(
        &self,
        filter: MatchFilter,
        attribute_name: String,
    ) -> anyhow::Result<ChartOverview> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::Chart(ChartCommand {
                filter,
                attribute_name,
                response: tx,
            })
        })
        .await
    }

    pub async fn enable_matchmaking(&self, enabled: bool) -> anyhow::Result<()> {
        self.send_command_and_await_for_result(move |tx| {
            ArenaCommand::EnableMatchmaking(EnableMatchmakingCommand {
                enabled,
                response: tx,
            })
        })
        .await
    }

    async fn send_command_and_await_for_result<R, F: FnOnce(oneshot::Sender<R>) -> ArenaCommand>(
        &self,
        cmd_builder: F,
    ) -> anyhow::Result<R> {
        let (tx, rx) = oneshot::channel();

        let cmd = cmd_builder(tx);

        self.commands_tx.send(cmd).await?;

        Ok(rx.await?)
    }
}
