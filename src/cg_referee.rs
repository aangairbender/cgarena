use std::path::PathBuf;

use anyhow::bail;
use itertools::Itertools;

pub struct CgReferee {
    git_url: String,
    fs_path: PathBuf,
}

impl CgReferee {
    pub fn new(git_url: String, fs_path: PathBuf) -> Self {
        Self { git_url, fs_path }
    }

    pub fn ensure_initialized(&self) -> anyhow::Result<()> {
        print!("Ensuring referee is cloned and patched... ");
        self.ensure_cloned()?;
        self.ensure_patched()?;
        self.ensure_built()?;
        println!(
            "Referee ready at {}",
            self.fs_path.join("target/referee.jar").to_string_lossy()
        );

        Ok(())
    }

    fn ensure_cloned(&self) -> anyhow::Result<()> {
        if self.fs_path.exists() {
            return Ok(());
        }

        eprintln!("{}", self.fs_path.to_str().unwrap());

        let output = std::process::Command::new("git")
            .arg("clone")
            .arg(&self.git_url)
            .arg(&self.fs_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            bail!("git clone failed")
        }
    }

    fn ensure_patched(&self) -> anyhow::Result<()> {
        let cli_path = self
            .fs_path
            .join("src/main/java/com/codingame/gameengine/runner/CommandLineInterface.java");
        if cli_path.exists() {
            return Ok(());
        }

        let cli_dir = self
            .fs_path
            .join("src/main/java/com/codingame/gameengine/runner");
        std::fs::create_dir_all(cli_dir)?;
        std::fs::write(cli_path, CLI_CONTENTS)?;

        let pom_path = self.fs_path.join("pom.xml");
        let pom = std::fs::read_to_string(&pom_path)?;
        let pom_lines = pom.lines().to_owned().collect_vec();
        if pom_lines.iter().any(|line| line.contains("<build>")) {
            return Ok(());
        }

        let Some(last_line_index) = pom_lines
            .iter()
            .position(|line| line.contains("</project>"))
        else {
            bail!("invalid pom.xml file: no '</projects>' terminator")
        };

        let mut new_pom = String::new();
        for i in 0..last_line_index {
            new_pom.push_str(pom_lines[i]);
        }
        new_pom.push_str(POM_BUILD_SECTION_CONTENTS);
        for i in last_line_index..pom_lines.len() {
            new_pom.push_str(pom_lines[i]);
        }
        std::fs::write(&pom_path, new_pom)?;

        Ok(())
    }

    fn ensure_built(&self) -> anyhow::Result<()> {
        let referee_path = self.fs_path.join("target/referee.jar");
        if referee_path.exists() {
            return Ok(());
        }

        let output = if cfg!(target_os = "windows") {
            // On Windows, use cmd /C to run the mvn command
            std::process::Command::new("cmd")
                .args(["/C", "mvn", "package"])
                .current_dir(&self.fs_path)
                .output()
        } else {
            // On Unix/Linux/macOS, call mvn directly
            std::process::Command::new("mvn")
                .args(["package"])
                .current_dir(&self.fs_path)
                .output()
        }?;

        if output.status.success() {
            Ok(())
        } else {
            bail!("mvn package failed")
        }
    }
}

static CLI_CONTENTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/CommandLineInterface.java"
));

static POM_BUILD_SECTION_CONTENTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/pom_build_section.xml"
));
