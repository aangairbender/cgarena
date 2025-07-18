use std::{ops::Deref, time::Duration};

use crate::{
    arena_handle::ArenaHandle,
    config::Config,
    db,
    domain::*,
    worker::{BuildBotInput, BuildBotOutput, PlayMatchOutput, WorkerHandle},
};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use tokio_util::sync::CancellationToken;

use crate::arena::*;
use crate::arena_commands::*;

struct TestArena {
    handle: ArenaHandle,
    cancellation_token: CancellationToken,
    pool: SqlitePool,
    match_result_tx: tokio::sync::mpsc::Sender<PlayMatchOutput>,
}

async fn create_test_arena<F1>(config: Config, builder: F1) -> TestArena
where
    F1: Fn(BuildBotInput) -> BuildResult + Send + 'static,
{
    let pool = db::in_memory().await.unwrap();
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

    // just dropping scheduled matches
    tokio::spawn(async move { while (match_rx.recv().await).is_some() {} });

    let handle = ArenaHandle::new(commands_tx);
    run(
        config.game,
        config.matchmaking,
        config.ranking,
        pool.clone(),
        worker_handle,
        commands_rx,
        cancellation_token.clone(),
    )
    .await
    .unwrap();

    TestArena {
        handle,
        cancellation_token,
        pool,
        match_result_tx,
    }
}

#[tokio::test]
async fn cmd_create_bot_should_create_record_in_db() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot) = res.unwrap() else {
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
    assert!(now < db_created_at + Duration::from_secs(1));

    arena.cancellation_token.cancel();
}

#[tokio::test]
async fn cmd_create_bot_should_fail_on_duplicate_name() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(_) = res.unwrap() else {
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

    let CreateBotResult::DuplicateName = res2.unwrap() else {
        panic!("Bot creation should fail with DuplicateName error")
    };
}

#[tokio::test]
async fn cmd_rename_bot_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot) = res.unwrap() else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena.handle.rename_bot(bot.id, bot_name_2.clone()).await;

    let RenameBotResult::Renamed = res2.unwrap() else {
        panic!("Bot renaming should succeed")
    };

    let row = sqlx::query("SELECT * FROM bots WHERE id = $1")
        .bind::<i64>(bot.id.into())
        .fetch_one(&arena.pool)
        .await
        .unwrap();

    let db_bot_name: String = row.get("name");
    assert_eq!(db_bot_name, bot_name_2.to_string());
}

#[tokio::test]
async fn cmd_fetch_bot_source_code_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot) = res.unwrap() else {
        panic!("Bot creation should succeed");
    };

    let res2 = arena
        .handle
        .fetch_bot_source_code(bot.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(res2.language, bot_language);
    assert_eq!(res2.source_code.deref(), bot_source_code.deref());
}

#[tokio::test]
async fn cmd_rename_bot_fails_on_duplicate_name() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot) = res.unwrap() else {
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

    let CreateBotResult::Created(_) = res2.unwrap() else {
        panic!("Bot creation should succeed");
    };

    let res3 = arena.handle.rename_bot(bot.id, bot_name_2.clone()).await;

    let RenameBotResult::DuplicateName = res3.unwrap() else {
        panic!("Bot renaming should fail with DuplicateName");
    };
}

#[tokio::test]
async fn cmd_rename_bot_fails_if_no_bot_with_id() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

    let bot_id: BotId = 1i64.into();
    let bot_name: BotName = String::from("Bot1").try_into().unwrap();

    let res = arena.handle.rename_bot(bot_id, bot_name.clone()).await;

    let RenameBotResult::NotFound = res.unwrap() else {
        panic!("Bot renaming should fail with NotFound");
    };
}

#[tokio::test]
async fn cmd_delete_bot_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot) = res.unwrap() else {
        panic!("Bot creation should succeed");
    };

    arena.handle.delete_bot(bot.id).await.unwrap();

    let row = sqlx::query("SELECT * FROM bots WHERE id = $1")
        .bind::<i64>(bot.id.into())
        .fetch_optional(&arena.pool)
        .await
        .unwrap();

    assert!(row.is_none());
}

#[tokio::test]
async fn cmd_fetch_leaderboard_works() {
    let config = Config::default();
    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot1) = res.unwrap() else {
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

    let CreateBotResult::Created(bot2) = res2.unwrap() else {
        panic!("Bot creation should succeed");
    };

    let res3 = arena.handle.fetch_status().await.unwrap();

    assert!(res3.matchmaking_enabled);
    assert_eq!(res3.bots.len(), 2);

    assert_eq!(res3.bots[0].id, bot1.id);
    assert_eq!(res3.bots[0].name, bot_name_1);
    assert_eq!(res3.bots[0].language, bot_language);
    assert_eq!(res3.bots[0].matches_played, 0);
    assert_eq!(res3.bots[0].matches_with_error, 0);
    assert!(res3.bots[0].builds.len() == 1);

    let build = &res3.bots[0].builds[0];
    assert_eq!(build.bot_id, bot1.id);
    assert_eq!(build.worker_name, WorkerName::embedded());
    assert!(build.was_finished_successfully());

    assert_eq!(res3.bots[1].id, bot2.id);
    assert_eq!(res3.bots[1].name, bot_name_2);
    assert_eq!(res3.bots[1].language, bot_language);
    assert_eq!(res3.bots[1].matches_played, 0);
    assert_eq!(res3.bots[1].matches_with_error, 0);
    assert!(res3.bots[1].builds.len() == 1);

    let build = &res3.bots[1].builds[0];
    assert_eq!(build.bot_id, bot2.id);
    assert_eq!(build.worker_name, WorkerName::embedded());
    assert!(build.was_finished_successfully());

    assert_eq!(res3.leaderboards.len(), 1);
    let leaderboard = &res3.leaderboards[0];

    assert_eq!(leaderboard.items.len(), 2);

    fn check_item(item: &LeaderboardItem, bot: BotOverview) {
        assert_eq!(item.id, bot.id);
        assert_eq!(item.rank, 0);
        assert!((item.rating.mu - 25.0).abs() < 0.001);
        assert!((item.rating.sigma - 8.3333).abs() < 0.001);
    }

    check_item(&leaderboard.items[0], bot1);
    check_item(&leaderboard.items[1], bot2);

    assert_eq!(leaderboard.winrate_stats.len(), 0);
    assert_eq!(leaderboard.total_matches, 0);
}

#[tokio::test]
async fn cmd_fetch_leaderboard_e2e() {
    let config = Config::default();

    let arena = create_test_arena(config, |_| BuildResult::Success).await;

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

    let CreateBotResult::Created(bot1) = res.unwrap() else {
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

    let CreateBotResult::Created(bot2) = res2.unwrap() else {
        panic!("Bot creation should succeed");
    };

    let b1 = bot1.id;
    let b2 = bot2.id;

    let fake_match_result = PlayMatchOutput {
        seed: 1234,
        participants: vec![
            Participant {
                bot_id: b1,
                rank: 0,
                error: false,
            },
            Participant {
                bot_id: b2,
                rank: 1,
                error: false,
            },
        ],
        attributes: {
            let mut initial = vec![
                MatchAttribute {
                    name: "initial_stones".to_string(),
                    bot_id: None,
                    turn: None,
                    value: "25".to_string().into(),
                },
                MatchAttribute {
                    name: "map_type".to_string(),
                    bot_id: None,
                    turn: None,
                    value: "small".to_string().into(),
                },
                MatchAttribute {
                    name: "stones_percentage".to_string(),
                    bot_id: None,
                    turn: None,
                    value: "0.75".to_string().into(),
                },
                MatchAttribute {
                    name: "final_score".to_string(),
                    bot_id: Some(b1),
                    turn: None,
                    value: "75".to_string().into(),
                },
                MatchAttribute {
                    name: "final_score".to_string(),
                    bot_id: Some(b2),
                    turn: None,
                    value: "50".to_string().into(),
                },
            ];

            for turn in 0..=5 {
                initial.push(MatchAttribute {
                    name: "bombs_revealed".to_string(),
                    bot_id: None,
                    turn: Some(turn),
                    value: (3 * turn).to_string().into(),
                });
                initial.push(MatchAttribute {
                    name: "score".to_string(),
                    bot_id: Some(b1),
                    turn: Some(turn),
                    value: (15 * turn).to_string().into(),
                });
                initial.push(MatchAttribute {
                    name: "score".to_string(),
                    bot_id: Some(b2),
                    turn: Some(turn),
                    value: (10 * turn).to_string().into(),
                });
            }

            initial
        },
    };
    arena.match_result_tx.send(fake_match_result).await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let res3 = arena.handle.fetch_status().await.unwrap();

    assert_eq!(res3.bots.len(), 2);
    assert_eq!(res3.bots[0].id, bot1.id);
    assert_eq!(res3.bots[0].matches_played, 1);
    assert_eq!(res3.bots[0].matches_with_error, 0);
    assert_eq!(res3.bots[1].id, bot2.id);
    assert_eq!(res3.bots[1].matches_played, 1);
    assert_eq!(res3.bots[1].matches_with_error, 0);

    let leaderboard = &res3.leaderboards[0];

    assert_eq!(leaderboard.items.len(), 2);

    let item1 = leaderboard.items.iter().find(|w| w.id == b1).unwrap();
    let item2 = leaderboard.items.iter().find(|w| w.id == b2).unwrap();

    assert_eq!(item1.rank, 0);
    assert_eq!(item2.rank, 1);

    assert!(item1.rating.score() > item2.rating.score());

    assert_eq!(leaderboard.winrate_stats[&(bot1.id, bot2.id)].wins, 1);
    assert_eq!(leaderboard.winrate_stats[&(bot1.id, bot2.id)].draws, 0);
    assert_eq!(leaderboard.winrate_stats[&(bot1.id, bot2.id)].loses, 0);
    assert_eq!(leaderboard.winrate_stats[&(bot2.id, bot1.id)].wins, 0);
    assert_eq!(leaderboard.winrate_stats[&(bot2.id, bot1.id)].draws, 0);
    assert_eq!(leaderboard.winrate_stats[&(bot2.id, bot1.id)].loses, 1);

    assert_eq!(leaderboard.total_matches, 1);
}
