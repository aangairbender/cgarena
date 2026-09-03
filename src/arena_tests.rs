use std::{fs, ops::Deref, time::Duration};

use crate::{
    arena_handle::ArenaHandle,
    config::{Config, WorkerConfig},
    db,
    domain::*,
    worker::{self, StartedWorker, WorkerSupervisor},
};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::arena::*;
use crate::arena_commands::*;

struct TestArena {
    handle: ArenaHandle,
    cancellation_token: CancellationToken,
    pool: SqlitePool,
    _arena_path: TempDir,
    _worker_supervisor: WorkerSupervisor,
    _arena_task: JoinHandle<anyhow::Result<()>>,
}

async fn create_test_arena(mut config: Config, play_output: Option<&str>) -> TestArena {
    let arena_path = tempfile::tempdir().unwrap();
    let play_script = arena_path.path().join("play-match.sh");
    let script = if let Some(output) = play_output {
        let marker = arena_path.path().join("first-match");
        format!(
            "#!/bin/sh\nif mkdir '{}' 2>/dev/null; then\ncat <<'EOF'\n{output}\nEOF\nelse\nexec sleep 30\nfi\n",
            marker.display()
        )
    } else {
        "#!/bin/sh\nexec sleep 30\n".to_string()
    };
    fs::write(&play_script, script).unwrap();

    let [WorkerConfig::Embedded(worker_config)] = config.workers.as_mut_slice() else {
        panic!("tests require one embedded worker");
    };
    worker_config.cmd_build = "sh -c true".to_string();
    worker_config.cmd_run = "true".to_string();
    worker_config.cmd_play_match =
        format!("sh {}", shell_words::quote(&play_script.to_string_lossy()));

    let StartedWorker { worker, supervisor } =
        worker::start_embedded_worker(arena_path.path(), worker_config.clone()).unwrap();
    let pool = db::in_memory().await.unwrap();
    let (commands_tx, commands_rx) = tokio::sync::mpsc::channel(16);
    let cancellation_token = CancellationToken::new();
    let handle = ArenaHandle::new(commands_tx);
    let arena_task = run(
        config.game,
        config.matchmaking,
        config.leaderboards,
        config.ranking,
        pool.clone(),
        arena_path.path().to_owned(),
        worker,
        commands_rx,
        cancellation_token.clone(),
    )
    .await
    .unwrap();

    TestArena {
        handle,
        cancellation_token,
        pool,
        _arena_path: arena_path,
        _worker_supervisor: supervisor,
        _arena_task: arena_task,
    }
}

#[tokio::test]
async fn cmd_create_bot_should_create_record_in_db() {
    let config = Config::default();
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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
    let arena = create_test_arena(config, None).await;

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

    let res3 = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = arena.handle.fetch_status().await.unwrap();
            if status.bots.len() == 2
                && status
                    .bots
                    .iter()
                    .all(|bot| bot.builds.len() == 1 && bot.builds[0].was_finished_successfully())
            {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("real build completions should reach Arena");

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
    }

    check_item(&leaderboard.items[0], bot1);
    check_item(&leaderboard.items[1], bot2);

    assert_eq!(leaderboard.winrate_stats.len(), 0);
    assert_eq!(leaderboard.total_matches, 0);
}

#[tokio::test]
async fn cmd_fetch_leaderboard_e2e() {
    let config = Config::default();
    let match_output = r#"{"ranks":[0,1],"errors":[0,0]}"#;
    let arena = create_test_arena(config, Some(match_output)).await;

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

    let res3 = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let status = arena.handle.fetch_status().await.unwrap();
            if status.leaderboards[0].total_matches == 1 {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("a real worker completion should reach Arena");

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

    let (winner, loser, winner_id, loser_id) = if item1.rank == 0 {
        (item1, item2, b1, b2)
    } else {
        (item2, item1, b2, b1)
    };
    assert_eq!(winner.rank, 0);
    assert_eq!(loser.rank, 1);
    assert!(winner.rating.score(3.0) > loser.rating.score(3.0));

    assert_eq!(leaderboard.winrate_stats[&(winner_id, loser_id)].wins, 1);
    assert_eq!(leaderboard.winrate_stats[&(winner_id, loser_id)].draws, 0);
    assert_eq!(leaderboard.winrate_stats[&(winner_id, loser_id)].loses, 0);
    assert_eq!(leaderboard.winrate_stats[&(loser_id, winner_id)].wins, 0);
    assert_eq!(leaderboard.winrate_stats[&(loser_id, winner_id)].draws, 0);
    assert_eq!(leaderboard.winrate_stats[&(loser_id, winner_id)].loses, 1);

    assert_eq!(leaderboard.total_matches, 1);
}
