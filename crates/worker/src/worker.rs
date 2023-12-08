use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::bail;
use config::WorkerConfig;
use entity::bot;
use tracing::info;

use super::{Job, JobResult, WorkerThread};

pub struct Worker {
    worker_threads: Vec<WorkerThread>,
    jobs_queued_total: usize,
    supported_languages: Vec<String>,
    known_bot_ids: Vec<i32>,
    languages_dir: PathBuf,
    bots_dir: PathBuf,
    matches_dir: PathBuf,
}

impl Worker {
    pub fn new(config: WorkerConfig) -> Result<Self, anyhow::Error> {
        assert!(config.threads > 0, "Can't start worker with 0 threads");
        let worker_threads = (0..config.threads)
            .map(|_| WorkerThread::spawn(config.clone()))
            .collect();
        info!("Worker with {} worker threads created", config.threads);

        let matches_dir = Path::new(&config.workdir).join("matches");
        if !matches_dir.exists() {
            fs::create_dir(&matches_dir)?;
        }

        // Detect supported languages
        let mut supported_languages = Vec::new();
        let languages_dir = PathBuf::from(&config.language_templates_path);
        if languages_dir.exists() {
            let paths = fs::read_dir(&languages_dir).unwrap();
            for path in paths {
                let path = path.unwrap();
                if path.metadata().unwrap().is_dir() {
                    supported_languages.push(path.file_name().to_str().unwrap().to_owned());
                }
            }
        } else {
            bail!("Language templates folder does not exist");
        }

        // Detect known bots
        let mut known_bot_ids: Vec<i32> = Vec::new();
        let bots_dir = Path::new(&config.workdir).join("bots");
        if bots_dir.exists() {
            let paths = fs::read_dir(&bots_dir).unwrap();
            for path in paths {
                let path = path.unwrap();
                if path.metadata().unwrap().is_dir() {
                    known_bot_ids.push(path.file_name().to_str().unwrap().parse().unwrap());
                }
            }
        } else {
            fs::create_dir(&bots_dir)?;
        }

        Ok(Self {
            worker_threads,
            jobs_queued_total: 0,
            supported_languages,
            known_bot_ids,
            languages_dir,
            bots_dir,
            matches_dir,
        })
    }

    pub async fn run(&mut self, job: Job) -> Result<JobResult, anyhow::Error> {
        for bot in &job.bots {
            if !self.known_bot_ids.contains(&bot.id) {
                self.build_bot(bot).await?;
                self.known_bot_ids.push(bot.id);
            }
        }

        let index = self.jobs_queued_total % self.worker_threads.len();
        self.jobs_queued_total += 1;
        // TODO: implement retry logic
        self.worker_threads[index].run(job).await
    }

    async fn build_bot(&mut self, bot: &bot::Model) -> Result<(), anyhow::Error> {
        let bot_dir = self.bots_dir.join(bot.id.to_string());
        copy_dir_all(self.languages_dir.join(&bot.language), &bot_dir)?;

        fs::write(bot_dir.join("main.txt"), &bot.source_code)?;
        while !bot_dir.join("main.txt").exists() {}
        let cmd = Command::new("sh")
            .arg(bot_dir.join("build.sh"))
            .current_dir(&bot_dir)
            .output()?;
        eprintln!("{:?}", &cmd);

        if cmd.status.success() {
            Ok(())
        } else {
            bail!(String::from_utf8(cmd.stdout).unwrap())
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
