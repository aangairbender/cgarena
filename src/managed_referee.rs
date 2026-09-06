use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::{fs, process::Command, sync::Mutex};
use uuid::Uuid;
use xmltree::{Element, XMLNode};

use crate::{config::ManagedCodingameRefereeConfig, referee_adapter::RefereeAdapter};

const ADAPTATION_SUBJECT: &str = "CG Arena referee adapter v1";
const CLI_PATH: &str = "src/main/java/com/codingame/gameengine/runner/CommandLineInterface.java";
const CLI_CONTENTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/CommandLineInterface.java"
));
const BUILD_FRAGMENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/pom_build_section.xml"
));

#[derive(Clone)]
pub struct ManagedReferee {
    arena_path: PathBuf,
    operation: Arc<Mutex<OperationStatus>>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct OperationStatus {
    pub action: Option<RefereeAction>,
    pub phase: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefereeAction {
    Install,
    Check,
    Rebuild,
    Update,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ManagedMetadata {
    pub repository_url: String,
    pub branch: String,
    pub upstream_commit: String,
    pub adaptation_commit: String,
    pub last_successful_check: Option<DateTime<Utc>>,
    pub observed_remote_commit: Option<String>,
    pub observed_ahead: Option<u32>,
    pub observed_behind: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RefereeStatus {
    pub selected: Option<ManagedCodingameRefereeConfig>,
    pub installed: bool,
    pub replacement_required: bool,
    pub checkout_path: String,
    pub artifact_path: String,
    pub branch: Option<String>,
    pub upstream_commit: Option<String>,
    pub installed_repository_url: Option<String>,
    pub adaptation_commit: Option<String>,
    pub committed_ahead: Option<u32>,
    pub committed_behind: Option<u32>,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub update_status: UpdateStatus,
    pub last_successful_check: Option<DateTime<Utc>>,
    pub observed_remote_commit: Option<String>,
    pub operation: OperationStatus,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    UpToDate,
    UpdateAvailable,
    Unavailable,
}

pub struct PreparedCandidate {
    pub artifact: PathBuf,
    pub checkout: Option<PathBuf>,
    pub metadata: ManagedMetadata,
    pub diagnostic: String,
}

pub enum ActionOutcome {
    Status(String),
    Candidate(PreparedCandidate),
}

impl ManagedReferee {
    pub fn new(arena_path: PathBuf) -> Self {
        Self {
            arena_path,
            operation: Arc::new(Mutex::new(OperationStatus::default())),
        }
    }

    pub async fn reserve(&self, action: RefereeAction) -> anyhow::Result<()> {
        let mut operation = self.operation.lock().await;
        if let Some(active) = operation.action {
            bail!("referee action {active:?} is already running");
        }
        *operation = OperationStatus {
            action: Some(action),
            phase: Some("queued".to_string()),
            diagnostic: None,
        };
        Ok(())
    }

    pub async fn phase(&self, phase: impl Into<String>) {
        self.operation.lock().await.phase = Some(phase.into());
    }

    pub async fn finish(&self, diagnostic: String) {
        *self.operation.lock().await = OperationStatus {
            action: None,
            phase: None,
            diagnostic: Some(diagnostic),
        };
    }

    pub async fn fail(&self, error: &anyhow::Error) {
        self.finish(format!("{error:#}")).await;
    }

    pub async fn selected_is_installed(
        &self,
        selected: &ManagedCodingameRefereeConfig,
    ) -> anyhow::Result<bool> {
        let Some(metadata) = self.load_metadata().await? else {
            return Ok(false);
        };
        Ok(metadata.repository_url == selected.repository_url
            && selected
                .branch
                .as_ref()
                .is_none_or(|branch| branch == &metadata.branch)
            && self.artifact_path().is_file())
    }

    pub async fn status(
        &self,
        selected: Option<ManagedCodingameRefereeConfig>,
    ) -> anyhow::Result<RefereeStatus> {
        let operation = self.operation.lock().await.clone();
        let metadata = self.load_metadata().await?;
        let installed =
            metadata.is_some() && self.checkout_path().is_dir() && self.artifact_path().is_file();
        let replacement_required = match (&selected, &metadata) {
            (Some(selected), Some(metadata)) => {
                selected.repository_url != metadata.repository_url
                    || selected
                        .branch
                        .as_ref()
                        .is_some_and(|branch| branch != &metadata.branch)
            }
            (Some(_), None) => false,
            _ => false,
        };
        let local = if operation.action.is_none() && self.checkout_path().is_dir() {
            local_status(&self.checkout_path(), metadata.as_ref())
                .await
                .unwrap_or_default()
        } else {
            LocalStatus::default()
        };
        let update_status = metadata
            .as_ref()
            .map_or(UpdateStatus::Unavailable, |metadata| {
                match (&metadata.observed_remote_commit, metadata.observed_behind) {
                    (Some(observed), _) if observed == &metadata.upstream_commit => {
                        UpdateStatus::UpToDate
                    }
                    (Some(_), Some(behind)) if behind > 0 => UpdateStatus::UpdateAvailable,
                    (Some(_), _) => UpdateStatus::UpToDate,
                    _ => UpdateStatus::Unavailable,
                }
            });
        Ok(RefereeStatus {
            selected,
            installed,
            replacement_required,
            checkout_path: self.checkout_path().display().to_string(),
            artifact_path: self.artifact_path().display().to_string(),
            installed_repository_url: metadata.as_ref().map(|value| value.repository_url.clone()),
            branch: metadata.as_ref().map(|value| value.branch.clone()),
            upstream_commit: metadata.as_ref().map(|value| value.upstream_commit.clone()),
            adaptation_commit: metadata
                .as_ref()
                .map(|value| value.adaptation_commit.clone()),
            committed_ahead: local.ahead,
            committed_behind: local.behind,
            staged: local.staged,
            unstaged: local.unstaged,
            untracked: local.untracked,
            update_status,
            last_successful_check: metadata
                .as_ref()
                .and_then(|value| value.last_successful_check),
            observed_remote_commit: metadata
                .as_ref()
                .and_then(|value| value.observed_remote_commit.clone()),
            operation,
        })
    }

    pub async fn execute(
        &self,
        action: RefereeAction,
        selected: &ManagedCodingameRefereeConfig,
    ) -> anyhow::Result<ActionOutcome> {
        match action {
            RefereeAction::Install => self.install(selected).await.map(ActionOutcome::Candidate),
            RefereeAction::Check => self.check(selected).await.map(ActionOutcome::Status),
            RefereeAction::Rebuild => self.rebuild(selected).await.map(ActionOutcome::Candidate),
            RefereeAction::Update => self.update(selected).await.map(ActionOutcome::Candidate),
        }
    }

    pub async fn publish_metadata(&self, metadata: &ManagedMetadata) -> anyhow::Result<()> {
        let directory = self.internal_path();
        fs::create_dir_all(&directory).await?;
        let pending = directory.join(format!("metadata-{}.json", Uuid::new_v4()));
        fs::write(&pending, serde_json::to_vec_pretty(metadata)?).await?;
        fs::rename(pending, self.metadata_path()).await?;
        Ok(())
    }
    pub async fn metadata_snapshot(&self) -> anyhow::Result<Option<Vec<u8>>> {
        match fs::read(self.metadata_path()).await {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub async fn restore_metadata(&self, snapshot: Option<Vec<u8>>) -> anyhow::Result<()> {
        match snapshot {
            Some(contents) => {
                fs::create_dir_all(self.internal_path()).await?;
                fs::write(self.metadata_path(), contents).await?;
            }
            None => match fs::remove_file(self.metadata_path()).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            },
        }
        Ok(())
    }

    pub fn checkout_path(&self) -> PathBuf {
        self.arena_path.join("referee")
    }

    pub fn artifact_path(&self) -> PathBuf {
        self.internal_path().join("referee.jar")
    }

    pub fn internal_path(&self) -> PathBuf {
        self.arena_path.join(".cgarena/referee")
    }

    async fn load_metadata(&self) -> anyhow::Result<Option<ManagedMetadata>> {
        match fs::read(self.metadata_path()).await {
            Ok(contents) => Ok(Some(
                serde_json::from_slice(&contents).context("invalid managed referee metadata")?,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn metadata_path(&self) -> PathBuf {
        self.internal_path().join("metadata.json")
    }

    async fn install(
        &self,
        selected: &ManagedCodingameRefereeConfig,
    ) -> anyhow::Result<PreparedCandidate> {
        if self.checkout_path().is_dir() {
            let local =
                local_status(&self.checkout_path(), self.load_metadata().await?.as_ref()).await?;
            if local.has_user_changes() {
                bail!("cannot replace the managed referee while its checkout has committed, staged, unstaged, or untracked changes; preserve or remove those changes with Git first");
            }
        }
        self.phase("cloning repository").await;
        let candidate_root = self
            .internal_path()
            .join(format!("candidate-{}", Uuid::new_v4()));
        let checkout = candidate_root.join("checkout");
        fs::create_dir_all(&candidate_root).await?;
        let mut arguments = vec![
            "clone".to_string(),
            "--origin".to_string(),
            "upstream".to_string(),
        ];
        if let Some(branch) = &selected.branch {
            arguments.extend([
                "--branch".to_string(),
                branch.clone(),
                "--single-branch".to_string(),
            ]);
        }
        arguments.extend([
            "--".to_string(),
            selected.repository_url.clone(),
            checkout.display().to_string(),
        ]);
        run(Command::new("git").args(&arguments), "git clone").await?;
        let branch = run_git(&checkout, ["branch", "--show-current"])
            .await?
            .trim()
            .to_string();
        if branch.is_empty() {
            bail!("the selected remote did not provide a default branch");
        }
        let upstream_commit = run_git(&checkout, ["rev-parse", "HEAD"])
            .await?
            .trim()
            .to_string();
        run_git(&checkout, ["checkout", "--detach", &upstream_commit]).await?;
        if !run_git(&checkout, ["branch", "--list", "cgarena"])
            .await?
            .trim()
            .is_empty()
        {
            run_git(&checkout, ["branch", "-D", "cgarena"]).await?;
        }
        run_git(&checkout, ["checkout", "-b", "cgarena"]).await?;
        self.phase("adapting referee").await;
        adapt_checkout(&checkout).await?;
        commit_adaptation(&checkout).await?;
        let adaptation_commit = run_git(&checkout, ["rev-parse", "HEAD"])
            .await?
            .trim()
            .to_string();
        let artifact = build_and_validate(&self.arena_path, &checkout, selected).await?;
        Ok(PreparedCandidate {
            artifact,
            checkout: Some(checkout),
            metadata: ManagedMetadata {
                repository_url: selected.repository_url.clone(),
                branch,
                upstream_commit,
                adaptation_commit,
                last_successful_check: None,
                observed_remote_commit: None,
                observed_ahead: None,
                observed_behind: None,
            },
            diagnostic: "Managed referee installed and activated".to_string(),
        })
    }

    async fn check(&self, selected: &ManagedCodingameRefereeConfig) -> anyhow::Result<String> {
        let mut metadata = self
            .load_metadata()
            .await?
            .context("no managed referee is installed")?;
        if metadata.repository_url != selected.repository_url
            || selected
                .branch
                .as_ref()
                .is_some_and(|branch| branch != &metadata.branch)
        {
            bail!(
                "the selected repository differs from the installed referee; use Replace referee"
            );
        }
        self.phase("fetching selected branch").await;
        run_git(
            &self.checkout_path(),
            ["fetch", "upstream", &metadata.branch],
        )
        .await?;
        let observed = run_git(&self.checkout_path(), ["rev-parse", "FETCH_HEAD"])
            .await?
            .trim()
            .to_string();
        let (behind, ahead) =
            divergence(&self.checkout_path(), &metadata.upstream_commit, &observed).await?;
        metadata.last_successful_check = Some(Utc::now());
        metadata.observed_remote_commit = Some(observed.clone());
        metadata.observed_ahead = Some(ahead);
        metadata.observed_behind = Some(behind);
        self.publish_metadata(&metadata).await?;
        Ok(if behind > 0 {
            format!(
                "Update available: installed base is {ahead} ahead and {behind} behind {observed}"
            )
        } else {
            format!("Referee is up to date at {observed}")
        })
    }

    async fn rebuild(
        &self,
        selected: &ManagedCodingameRefereeConfig,
    ) -> anyhow::Result<PreparedCandidate> {
        let metadata = self
            .load_metadata()
            .await?
            .context("no managed referee is installed")?;
        if metadata.repository_url != selected.repository_url
            || selected
                .branch
                .as_ref()
                .is_some_and(|branch| branch != &metadata.branch)
        {
            bail!(
                "the selected repository differs from the installed referee; use Replace referee"
            );
        }
        self.phase("refreshing local adaptation").await;
        adapt_checkout(&self.checkout_path()).await?;
        let artifact =
            build_and_validate(&self.arena_path, &self.checkout_path(), selected).await?;
        Ok(PreparedCandidate {
            artifact,
            checkout: None,
            metadata,
            diagnostic: "Local referee changes rebuilt and activated".to_string(),
        })
    }

    async fn update(
        &self,
        selected: &ManagedCodingameRefereeConfig,
    ) -> anyhow::Result<PreparedCandidate> {
        let mut metadata = self
            .load_metadata()
            .await?
            .context("no managed referee is installed")?;
        let observed = metadata
            .observed_remote_commit
            .clone()
            .context("check for updates successfully before updating")?;
        if observed == metadata.upstream_commit {
            bail!("the managed referee is already up to date");
        }
        if metadata.repository_url != selected.repository_url
            || selected
                .branch
                .as_ref()
                .is_some_and(|branch| branch != &metadata.branch)
        {
            bail!(
                "the selected repository differs from the installed referee; use Replace referee"
            );
        }
        self.phase("preserving working tree changes").await;
        let checkout = self.checkout_path();
        let rebase_in_progress = checkout.join(".git/rebase-merge").is_dir()
            || checkout.join(".git/rebase-apply").is_dir();
        if rebase_in_progress {
            run_git(&checkout, ["rebase", "--continue"]).await.context(
                "cannot continue replaying user commits; resolve or abort the standard Git conflict, then retry",
            )?;
        } else {
            let dirty = local_status(&checkout, Some(&metadata))
                .await?
                .has_worktree_changes();
            if dirty {
                run_git(
                    &checkout,
                    [
                        "stash",
                        "push",
                        "--include-untracked",
                        "--message",
                        "cgarena-managed-update",
                    ],
                )
                .await?;
            }
            run_git(
                &checkout,
                [
                    "rebase",
                    "--onto",
                    &observed,
                    &metadata.upstream_commit,
                    "cgarena",
                ],
            )
            .await
            .context(
                "cannot replay managed and user commits; resolve or abort the standard Git conflict, then retry",
            )?;
        }
        let adaptation_commit = run_git(
            &checkout,
            [
                "log",
                "--reverse",
                "--format=%H",
                &format!("--grep=^{ADAPTATION_SUBJECT}$"),
                &format!("{}..HEAD", observed),
            ],
        )
        .await?
        .lines()
        .next()
        .context("the updated branch lost the identifiable CG Arena adaptation commit")?
        .to_string();
        let stashes = run_git(&checkout, ["stash", "list", "--format=%s"]).await?;
        if stashes
            .lines()
            .any(|line| line.contains("cgarena-managed-update"))
        {
            run_git(&checkout, ["stash", "pop"]).await.context(
                "updated the managed branch but could not restore working-tree changes; resolve the standard Git conflict, then retry",
            )?;
        }
        let artifact = build_and_validate(&self.arena_path, &checkout, selected).await?;
        metadata.upstream_commit = observed;
        metadata.adaptation_commit = adaptation_commit;
        Ok(PreparedCandidate {
            artifact,
            checkout: None,
            metadata,
            diagnostic: "Managed referee updated and activated".to_string(),
        })
    }
}

#[derive(Default)]
struct LocalStatus {
    ahead: Option<u32>,
    behind: Option<u32>,
    staged: bool,
    unstaged: bool,
    untracked: bool,
}

impl LocalStatus {
    fn has_worktree_changes(&self) -> bool {
        self.staged || self.unstaged || self.untracked
    }

    fn has_user_changes(&self) -> bool {
        self.ahead.unwrap_or(0) > 1 || self.behind.unwrap_or(0) > 0 || self.has_worktree_changes()
    }
}

async fn local_status(
    path: &Path,
    metadata: Option<&ManagedMetadata>,
) -> anyhow::Result<LocalStatus> {
    let porcelain = run_git(path, ["status", "--porcelain=v1"]).await?;
    let mut status = LocalStatus::default();
    for line in porcelain.lines() {
        let bytes = line.as_bytes();
        if line.starts_with("??") {
            status.untracked = true;
        } else if bytes.len() >= 2 {
            status.staged |= bytes[0] != b' ';
            status.unstaged |= bytes[1] != b' ';
        }
    }
    if let Some(metadata) = metadata {
        let (ahead, behind) = divergence(path, &metadata.upstream_commit, "HEAD").await?;
        status.ahead = Some(ahead);
        status.behind = Some(behind);
    }
    Ok(status)
}

async fn divergence(path: &Path, left: &str, right: &str) -> anyhow::Result<(u32, u32)> {
    let output = run_git(
        path,
        [
            "rev-list",
            "--left-right",
            "--count",
            &format!("{left}...{right}"),
        ],
    )
    .await?;
    let mut fields = output.split_whitespace();
    let left_only = fields
        .next()
        .context("Git omitted left divergence")?
        .parse()?;
    let right_only = fields
        .next()
        .context("Git omitted right divergence")?
        .parse()?;
    Ok((right_only, left_only))
}

async fn adapt_checkout(checkout: &Path) -> anyhow::Result<()> {
    let exclude = checkout.join(".git/info/exclude");
    if exclude.is_file() {
        let mut contents = fs::read_to_string(&exclude).await?;
        if !contents.lines().any(|line| line.trim() == "/target/") {
            contents.push_str("\n/target/\n");
            fs::write(exclude, contents).await?;
        }
    }
    let pom_path = checkout.join("pom.xml");
    if !pom_path.is_file() {
        bail!("unsupported CodinGame referee structure: expected pom.xml");
    }
    let cli_path = checkout.join(CLI_PATH);
    let cli_parent = cli_path
        .parent()
        .context("maintained referee CLI path has no parent directory")?;
    fs::create_dir_all(cli_parent).await?;
    let cli = match fs::read_to_string(&cli_path).await {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if cli.is_none_or(|contents| !contents.contains("CGARENA_COMPATIBILITY_VERSION")) {
        fs::write(&cli_path, CLI_CONTENTS).await?;
    }
    let source = fs::read(&pom_path).await?;
    let mut project = Element::parse(source.as_slice()).context("cannot parse referee pom.xml")?;
    if project.name != "project" {
        bail!("unsupported Maven structure: pom.xml root must be project");
    }
    let schema_location_prefix = project.namespaces.as_ref().and_then(|namespaces| {
        namespaces.0.iter().find_map(|(prefix, namespace)| {
            (!prefix.is_empty() && namespace == "http://www.w3.org/2001/XMLSchema-instance")
                .then(|| prefix.clone())
        })
    });
    if let (Some(prefix), Some(value)) = (
        schema_location_prefix,
        project.attributes.remove("schemaLocation"),
    ) {
        project
            .attributes
            .insert(format!("{prefix}:schemaLocation"), value);
    }
    let dependencies = child_mut(&mut project, "dependencies")
        .context("unsupported Maven structure: project/dependencies is required")?;
    if !contains_artifact(dependencies, "commons-cli") {
        dependencies.children.push(XMLNode::Element(parse_element(
            "<dependency><groupId>commons-cli</groupId><artifactId>commons-cli</artifactId><version>1.3.1</version></dependency>",
        )?));
    }
    let fragment = Element::parse(BUILD_FRAGMENT.as_bytes())
        .context("invalid maintained Maven build fragment")?;
    let build = ensure_child(&mut project, "build");
    let plugins = ensure_child(build, "plugins");
    let maintained = fragment
        .get_child("plugins")
        .context("maintained build fragment omits plugins")?;
    let existing = plugins
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .filter_map(|plugin| plugin.get_child("artifactId"))
        .filter_map(Element::get_text)
        .map(|value| value.to_string())
        .collect::<HashSet<_>>();
    for plugin in maintained.children.iter().filter_map(XMLNode::as_element) {
        let Some(artifact) = plugin.get_child("artifactId").and_then(Element::get_text) else {
            continue;
        };
        if !existing.contains(artifact.as_ref()) {
            plugins.children.push(XMLNode::Element(plugin.clone()));
        }
    }
    let mut rendered = Vec::new();
    project.write(&mut rendered)?;
    fs::write(pom_path, rendered).await?;
    Ok(())
}

fn child_mut<'a>(element: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    element.children.iter_mut().find_map(|child| match child {
        XMLNode::Element(child) if child.name == name => Some(child),
        _ => None,
    })
}

fn ensure_child<'a>(element: &'a mut Element, name: &str) -> &'a mut Element {
    if child_mut(element, name).is_none() {
        element.children.push(XMLNode::Element(Element::new(name)));
    }
    child_mut(element, name).expect("child was just inserted")
}

fn contains_artifact(element: &Element, artifact: &str) -> bool {
    element
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .any(|dependency| {
            dependency
                .get_child("artifactId")
                .and_then(Element::get_text)
                .is_some_and(|value| value == artifact)
        })
}

fn parse_element(source: &str) -> anyhow::Result<Element> {
    Element::parse(source.as_bytes()).map_err(Into::into)
}

async fn commit_adaptation(checkout: &Path) -> anyhow::Result<()> {
    run_git(checkout, ["add", "pom.xml", CLI_PATH]).await?;
    run_git(
        checkout,
        [
            "-c",
            "user.name=CG Arena",
            "-c",
            "user.email=cgarena@localhost",
            "commit",
            "--allow-empty",
            "-m",
            ADAPTATION_SUBJECT,
        ],
    )
    .await?;
    Ok(())
}

async fn build_and_validate(
    arena_path: &Path,
    checkout: &Path,
    selected: &ManagedCodingameRefereeConfig,
) -> anyhow::Result<PathBuf> {
    let wrapper = checkout.join("mvnw");
    let mut command = if wrapper.is_file() {
        let mut command = Command::new(&wrapper);
        command.current_dir(checkout);
        command
    } else {
        let mut command = Command::new(selected.maven.as_deref().unwrap_or("mvn"));
        command.current_dir(checkout);
        command
    };
    command.args(["--batch-mode", "-DskipTests", "package"]);
    run(&mut command, "Maven referee build").await?;
    let artifact = find_jar(&checkout.join("target")).await?;
    RefereeAdapter::codingame_candidate(
        artifact.clone(),
        selected.java.clone().unwrap_or_else(|| "java".to_string()),
    )
    .validate_startup(arena_path)
    .await?;
    Ok(artifact)
}

async fn find_jar(target: &Path) -> anyhow::Result<PathBuf> {
    let maintained = target.join("referee.jar");
    if maintained.is_file() {
        return Ok(maintained);
    }
    let mut entries = fs::read_dir(target)
        .await
        .with_context(|| format!("Maven did not create {}", target.display()))?;
    let mut jars = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if path.extension() == Some(OsStr::new("jar"))
            && !name.starts_with("original-")
            && !name.ends_with("-sources.jar")
            && !name.ends_with("-javadoc.jar")
        {
            jars.push(path);
        }
    }
    jars.sort();
    match jars.as_slice() {
        [artifact] => Ok(artifact.clone()),
        [] => bail!(
            "Maven build produced no runnable JAR in {}",
            target.display()
        ),
        _ => bail!(
            "Maven build produced multiple candidate JARs in {}: {}",
            target.display(),
            jars.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

async fn run_git<I, S>(path: &Path, arguments: I) -> anyhow::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command
        .args(arguments)
        .current_dir(path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true");
    run(&mut command, "Git operation").await
}

async fn run(command: &mut Command, description: &str) -> anyhow::Result<String> {
    command.kill_on_drop(true);
    let rendered = format!("{command:?}");
    let output = command
        .output()
        .await
        .with_context(|| format!("cannot start {description}: {rendered}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        bail!(
            "{description} failed ({rendered}, status {}): {}",
            output.status,
            if stderr.is_empty() { &stdout } else { &stderr }
        );
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn git(path: &Path, arguments: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(arguments)
            .current_dir(path)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_EDITOR", "true")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn commit(path: &Path, message: &str) {
        git(path, &["add", "."]);
        git(
            path,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                message,
            ],
        );
    }

    #[tokio::test]
    async fn find_jar_prefers_maintained_referee_artifact() {
        let target = tempfile::tempdir().unwrap();
        let referee = target.path().join("referee.jar");
        std::fs::write(&referee, "maintained").unwrap();
        std::fs::write(
            target
                .path()
                .join("summer-challenge-2025-super-soaker-1.0-SNAPSHOT.jar"),
            "original",
        )
        .unwrap();

        assert_eq!(find_jar(target.path()).await.unwrap(), referee);
    }

    #[tokio::test]
    async fn adaptation_preserves_namespaced_schema_location() {
        let checkout = tempfile::tempdir().unwrap();
        std::fs::write(
            checkout.path().join("pom.xml"),
            r#"<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd"><modelVersion>4.0.0</modelVersion><dependencies></dependencies></project>"#,
        )
        .unwrap();

        adapt_checkout(checkout.path()).await.unwrap();

        let adapted = std::fs::read_to_string(checkout.path().join("pom.xml")).unwrap();
        assert!(
            adapted.contains(r#"xsi:schemaLocation=""#),
            "adapted POM lost the xsi namespace prefix:\n{adapted}"
        );
        assert!(
            !adapted.contains(r#" schemaLocation=""#),
            "adapted POM contains Maven's invalid unqualified schemaLocation:\n{adapted}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn install_adapts_standard_referee_without_command_line_interface() {
        let arena = tempfile::tempdir().unwrap();
        let source = arena.path().join("source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("pom.xml"),
            "<project><modelVersion>4.0.0</modelVersion><dependencies></dependencies></project>",
        )
        .unwrap();
        std::fs::write(
            source.join("mvnw"),
            "#!/bin/sh\nset -eu\nmkdir -p target\nprintf fixture > target/referee.jar\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(source.join("mvnw"))
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(source.join("mvnw"), permissions).unwrap();
        git(&source, &["init", "-b", "trunk"]);
        commit(&source, "initial");

        let java = arena.path().join("java");
        std::fs::write(&java, "#!/bin/sh\nprintf '%s\\n' cgarena-referee-v1\n").unwrap();
        let mut permissions = std::fs::metadata(&java).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&java, permissions).unwrap();
        let selected = ManagedCodingameRefereeConfig {
            repository_url: source.display().to_string(),
            branch: None,
            java: Some(java.display().to_string()),
            maven: None,
        };

        let manager = ManagedReferee::new(arena.path().to_owned());
        let ActionOutcome::Candidate(installed) = manager
            .execute(RefereeAction::Install, &selected)
            .await
            .expect("a standard referee must be adapted during installation")
        else {
            panic!("install must produce a candidate");
        };
        let checkout = installed
            .checkout
            .expect("install must retain its checkout");
        assert_eq!(
            std::fs::read_to_string(checkout.join(CLI_PATH)).unwrap(),
            CLI_CONTENTS
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn update_leaves_standard_conflicts_and_retry_continues_after_resolution() {
        let arena = tempfile::tempdir().unwrap();
        let source = arena.path().join("source");
        std::fs::create_dir_all(source.join("src/main/java/com/codingame/gameengine/runner"))
            .unwrap();
        std::fs::write(
            source.join("pom.xml"),
            "<project><modelVersion>4.0.0</modelVersion><dependencies></dependencies></project>",
        )
        .unwrap();
        std::fs::write(
            source.join(CLI_PATH),
            "package com.codingame.gameengine.runner; public class CommandLineInterface {}",
        )
        .unwrap();
        std::fs::write(source.join("README"), "base\n").unwrap();
        std::fs::write(
            source.join("mvnw"),
            "#!/bin/sh\nset -eu\nmkdir -p target\nprintf fixture > target/referee.jar\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(source.join("mvnw"))
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(source.join("mvnw"), permissions).unwrap();
        git(&source, &["init", "-b", "trunk"]);
        commit(&source, "initial");

        let java = arena.path().join("java");
        std::fs::write(&java, "#!/bin/sh\nprintf '%s\\n' cgarena-referee-v1\n").unwrap();
        let mut permissions = std::fs::metadata(&java).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&java, permissions).unwrap();
        let selected = ManagedCodingameRefereeConfig {
            repository_url: source.display().to_string(),
            branch: None,
            java: Some(java.display().to_string()),
            maven: None,
        };
        let manager = ManagedReferee::new(arena.path().to_owned());
        let ActionOutcome::Candidate(installed) = manager
            .execute(RefereeAction::Install, &selected)
            .await
            .unwrap()
        else {
            panic!("install must produce a candidate");
        };
        let artifact = std::fs::read(&installed.artifact).unwrap();
        std::fs::rename(installed.checkout.unwrap(), manager.checkout_path()).unwrap();
        std::fs::create_dir_all(manager.internal_path()).unwrap();
        std::fs::write(manager.artifact_path(), artifact).unwrap();
        manager.publish_metadata(&installed.metadata).await.unwrap();

        let checkout = manager.checkout_path();
        std::fs::write(checkout.join("README"), "user\n").unwrap();
        commit(&checkout, "user change");
        std::fs::write(source.join("README"), "upstream\n").unwrap();
        std::fs::write(
            source.join(CLI_PATH),
            "package com.codingame.gameengine.runner; public class CommandLineInterface { int upstream; }",
        )
        .unwrap();
        commit(&source, "conflicting upstream change");
        manager
            .execute(RefereeAction::Check, &selected)
            .await
            .unwrap();

        let Err(adaptation_conflict) = manager.execute(RefereeAction::Update, &selected).await
        else {
            panic!("adaptation conflict must stop the update");
        };
        assert!(format!("{adaptation_conflict:#}").contains("standard Git conflict"));
        assert!(checkout.join(".git/rebase-merge").is_dir());
        std::fs::write(checkout.join(CLI_PATH), CLI_CONTENTS).unwrap();
        git(&checkout, &["add", CLI_PATH]);

        let Err(user_conflict) = manager.execute(RefereeAction::Update, &selected).await else {
            panic!("user conflict must stop the update");
        };
        assert!(format!("{user_conflict:#}").contains("standard Git conflict"));
        assert!(checkout.join(".git/rebase-merge").is_dir());
        std::fs::write(checkout.join("README"), "resolved\n").unwrap();
        git(&checkout, &["add", "README"]);

        let ActionOutcome::Candidate(updated) = manager
            .execute(RefereeAction::Update, &selected)
            .await
            .unwrap()
        else {
            panic!("resolved update must produce a candidate");
        };
        assert!(!checkout.join(".git/rebase-merge").exists());
        assert_eq!(
            std::fs::read_to_string(checkout.join("README")).unwrap(),
            "resolved\n"
        );
        assert_eq!(
            updated.metadata.upstream_commit,
            git(&source, &["rev-parse", "HEAD"])
        );
    }
}
