use crate::{
    arena_handle::ArenaHandle,
    config::Config,
    db::Database,
    domain::*,
    worker::{BuildBotInput, BuildBotOutput, PlayMatchInput, PlayMatchOutput, WorkerHandle},
};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
use sqlx::{Row, SqlitePool};
use tokio_util::sync::CancellationToken;

use crate::arena::*;

struct TestArena {
    handle: ArenaHandle,
    cancellation_token: CancellationToken,
    pool: SqlitePool,
}

async fn create_test_arena<F1, F2>(config: Config, builder: F1, runner: F2) -> TestArena
where
    F1: Fn(BuildBotInput) -> BuildResult + Send + 'static,
    F2: Fn(PlayMatchInput) -> PlayMatchOutput + Send + 'static,
{
    let (db, pool) = Database::in_memory().await;
    let (commands_tx, commands_rx) = tokio::sync::mpsc::channel(16);
    let cancellation_token = CancellationToken::new();
    let (match_result_tx, match_result_rx) = tokio::sync::mpsc::channel(100);
    let (match_tx, mut match_rx) = tokio::sync::mpsc::channel(16);
    let (build_tx, mut build_rx) = tokio::sync::mpsc::channel(1);
    let worker_handle = WorkerHandle {
        match_tx,
        match_result_rx,
        build_tx,
        known_bot_ids: vec![],
    };

    tokio::spawn(async move {
        while let Some(cmd) = build_rx.recv().await {
            let output = BuildBotOutput {
                bot_id: cmd.input.bot_id,
                worker_name: cmd.input.worker_name.clone(),
                result: builder(cmd.input),
            };
            cmd.result.send(output).unwrap();
        }
    });

    tokio::spawn(async move {
        while let Some(input) = match_rx.recv().await {
            let output = runner(input);
            match_result_tx.send(output).await.unwrap();
        }
    });

    let handle = ArenaHandle::new(commands_tx);
    tokio::spawn(run(
        config.game,
        config.matchmaking,
        config.ranking,
        db,
        worker_handle,
        commands_rx,
        cancellation_token.clone(),
    ));

    TestArena {
        handle,
        cancellation_token,
        pool,
    }
}

fn lowest_id_wins(input: PlayMatchInput) -> PlayMatchOutput {
    PlayMatchOutput {
        seed: input.seed,
        participants: input
            .bots
            .into_iter()
            .sorted_by_key(|b| Into::<i64>::into(b.bot_id))
            .enumerate()
            .map(|(i, b)| Participant {
                bot_id: b.bot_id,
                rank: i as u8,
                error: false,
            })
            .collect_vec(),
        attributes: MatchAttributes::default(),
    }
}

#[tokio::test]
async fn cmd_create_bot_should_create_record_in_db() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();
    let now = Utc::now();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot) = res else {
        panic!("Bot creation should succeed");
    };

    assert_ne!(bot.id, BotId::UNINITIALIZED);
    assert_eq!(bot.name, bot_name);

    let row = sqlx::query("SELECT * FROM bots WHERE id = $1")
        .bind::<i64>(bot.id.into())
        .fetch_one(&arena.pool)
        .await
        .unwrap();

    let res_bot_id: i64 = bot.id.into();
    let db_bot_id: i64 = row.get("id");
    assert_eq!(db_bot_id, res_bot_id);

    let db_bot_name: String = row.get("name");
    assert_eq!(db_bot_name, bot_name.to_string());

    let db_source_code: String = row.get("source_code");
    assert_eq!(db_source_code, bot_source_code.to_string());

    let db_language: String = row.get("language");
    assert_eq!(db_language, bot_language.to_string());

    let db_created_at: DateTime<Utc> = row.get("created_at");
    assert!(db_created_at > now);
    assert!(now < db_created_at + Duration::seconds(1));

    arena.cancellation_token.cancel();
}

#[tokio::test]
async fn cmd_create_bot_should_fail_on_duplicate_name() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(_) = res else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::DuplicateName = res2 else {
        panic!("Bot creation should fail with DuplicateName error")
    };
}

#[tokio::test]
async fn cmd_rename_bot_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_name_2: BotName = String::from("Bot2").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot) = res else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena.handle.rename_bot(bot.id, bot_name_2.clone()).await;

    let RenameBotResult::Renamed(bot2) = res2 else {
        panic!("Bot renaming should succeed")
    };

    assert_eq!(bot2.id, bot.id);
    assert_eq!(bot2.name, bot_name_2);

    let row = sqlx::query("SELECT * FROM bots WHERE id = $1")
        .bind::<i64>(bot.id.into())
        .fetch_one(&arena.pool)
        .await
        .unwrap();

    let db_bot_name: String = row.get("name");
    assert_eq!(db_bot_name, bot_name_2.to_string());
}

#[tokio::test]
async fn cmd_rename_bot_fails_on_duplicate_name() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_name_2: BotName = String::from("Bot2").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot) = res else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena
        .handle
        .create_bot(
            bot_name_2.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(_) = res2 else {
        panic!("Bot creation should succeed");
    };

    let res3 = arena.handle.rename_bot(bot.id, bot_name_2.clone()).await;

    let RenameBotResult::DuplicateName = res3 else {
        panic!("Bot renaming should fail with DuplicateName");
    };
}

#[tokio::test]
async fn cmd_rename_bot_fails_if_no_bot_with_id() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_id: BotId = 1i64.into();
    let bot_name: BotName = String::from("Bot1").try_into().unwrap();

    let res = arena.handle.rename_bot(bot_id, bot_name.clone()).await;

    let RenameBotResult::NotFound = res else {
        panic!("Bot renaming should fail with NotFound");
    };
}

#[tokio::test]
async fn cmd_delete_bot_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot) = res else {
        panic!("Bot creation should succeed");
    };

    arena.handle.delete_bot(bot.id).await;

    let row = sqlx::query("SELECT * FROM bots WHERE id = $1")
        .bind::<i64>(bot.id.into())
        .fetch_optional(&arena.pool)
        .await
        .unwrap();

    assert!(row.is_none());
}

#[tokio::test]
async fn cmd_fetch_all_bots_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name: BotName = String::from("Bot1").try_into().unwrap();
    let bot_name_2: BotName = String::from("Bot2").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot1) = res else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena
        .handle
        .create_bot(
            bot_name_2.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot2) = res2 else {
        panic!("Bot creation should succeed");
    };

    let res3 = arena.handle.fetch_all_bots().await;

    let expected = vec![
        BotMinimal {
            id: bot2.id,
            name: bot_name_2,
        },
        BotMinimal {
            id: bot1.id,
            name: bot_name,
        },
    ];

    assert_eq!(expected, res3);
}

#[tokio::test]
async fn cmd_fetch_leaderboard_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success, lowest_id_wins).await;

    let bot_name_1: BotName = String::from("Bot1").try_into().unwrap();
    let bot_name_2: BotName = String::from("Bot2").try_into().unwrap();
    let bot_source_code: SourceCode = String::from("some code").try_into().unwrap();
    let bot_language: Language = String::from("rust").try_into().unwrap();

    let res = arena
        .handle
        .create_bot(
            bot_name_1.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot1) = res else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena
        .handle
        .create_bot(
            bot_name_2.clone(),
            bot_source_code.clone(),
            bot_language.clone(),
        )
        .await;

    let CreateBotResult::Created(bot2) = res2 else {
        panic!("Bot creation should succeed");
    };

    let res3 = arena.handle.fetch_leaderboard(bot1.id).await;

    let Some(res3) = res3 else {
        panic!("Should return a leaderboard");
    };

    assert_eq!(res3.bot_overview.id, bot1.id);
    assert_eq!(res3.bot_overview.name, bot_name_1);
    assert_eq!(res3.bot_overview.language, bot_language);
    assert!((res3.bot_overview.rating.mu - 25.0).abs() < 0.001);
    assert!((res3.bot_overview.rating.sigma - 8.3333).abs() < 0.001);
    assert_eq!(res3.bot_overview.matches_played, 0);
    assert_eq!(res3.bot_overview.matches_with_error, 0);
    assert!(res3.bot_overview.builds.len() == 1);

    let build = &res3.bot_overview.builds[0];
    assert_eq!(build.bot_id, bot1.id);
    assert_eq!(build.worker_name, WorkerName::embedded());
    assert!(build.was_finished_successfully());

    assert_eq!(res3.items.len(), 2);

    fn check_item(item: &LeaderboardItem, bot: BotMinimal, rank: usize) {
        assert_eq!(item.id, bot.id);
        assert_eq!(item.name, bot.name);
        assert_eq!(item.rank, rank);
        assert!((item.rating.mu - 25.0).abs() < 0.001);
        assert!((item.rating.sigma - 8.3333).abs() < 0.001);
        assert_eq!(item.wins, 0);
        assert_eq!(item.loses, 0);
        assert_eq!(item.draws, 0);
    }

    check_item(&res3.items[0], bot1, 1);
    check_item(&res3.items[1], bot2, 1);
}
