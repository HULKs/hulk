use std::{ffi::OsStr, io::ErrorKind, path::Path, process::ExitStatus};

use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use tokio::process::Command;

struct GitCommand {
    inner: Command,
}

impl GitCommand {
    fn new(root: &Path, subcommand: impl AsRef<OsStr>) -> Self {
        let mut inner = Command::new("git");
        inner.current_dir(root).arg(subcommand);

        Self { inner }
    }

    fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.inner.arg(arg);
        self
    }

    fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.inner.args(args);
        self
    }

    async fn output(mut self) -> Result<String> {
        let output = self.inner.output().await.wrap_err("failed to run git")?;
        if !output.status.success() {
            bail!("git failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        String::from_utf8(output.stdout).wrap_err("git returned invalid UTF-8")
    }

    async fn status(mut self) -> Result<ExitStatus> {
        self.inner.status().await.wrap_err("failed to run git")
    }

    async fn run(self) -> Result<()> {
        let status = self.status().await?;

        if !status.success() {
            bail!("git failed with {status}");
        }

        Ok(())
    }
}

pub async fn create_and_switch_to_branch(
    root: &Path,
    name: &str,
    base: &str,
    force: bool,
) -> Result<()> {
    let create_flag = if force { "--force-create" } else { "--create" };

    GitCommand::new(root, "switch")
        .arg("--no-track")
        .arg(create_flag)
        .arg(name)
        .arg(base)
        .run()
        .await
}

pub async fn create_commit(root: &Path, message: &str) -> Result<()> {
    GitCommand::new(root, "commit")
        .arg("--all")
        .arg("--allow-empty")
        .arg("--message")
        .arg(message)
        .run()
        .await
}

pub async fn add_all(root: &Path) -> Result<()> {
    GitCommand::new(root, "add").arg(".").run().await
}

pub async fn reset(root: &Path, revision: &str, soft: bool) -> Result<()> {
    GitCommand::new(root, "reset")
        .arg(if soft { "--soft" } else { "--hard" })
        .arg(revision)
        .run()
        .await
}

pub async fn merge_squash(root: &Path, branch: &str) -> Result<()> {
    GitCommand::new(root, "merge")
        .arg("--squash")
        .arg(branch)
        .run()
        .await
}

pub async fn resolve_revision(root: &Path, revision: &str) -> Result<String> {
    let output = GitCommand::new(root, "rev-parse")
        .arg("--verify")
        .arg("--end-of-options")
        .arg(format!("{revision}^{{commit}}"))
        .output()
        .await?;
    Ok(output.trim().to_owned())
}

pub async fn select_commits(
    root: &Path,
    revisions: &[&str],
    top: Option<usize>,
) -> Result<Vec<String>> {
    let mut missing = Vec::new();
    for revision in revisions {
        if GitCommand::new(root, "rev-list")
            .args(["--no-walk=unsorted", "--max-count=0", revision, "--"])
            .output()
            .await
            .is_err()
        {
            missing.push(*revision);
        }
    }
    if !missing.is_empty() {
        bail!(
            "can't find revisions to cherry-pick: {}",
            missing.join(", ")
        );
    }
    let mut command = GitCommand::new(root, "rev-list");
    if let Some(count) = top {
        command = command
            .arg("--first-parent")
            .arg(format!("--max-count={count}"));
    } else {
        // Git expands ranges and exclusions to negative revisions, which enable a history walk.
        let expanded = GitCommand::new(root, "rev-parse")
            .args(["--revs-only", "--end-of-options"])
            .args(revisions)
            .output()
            .await?;
        command = command.arg("--no-walk=unsorted");
        if expanded.lines().any(|revision| revision.starts_with('^')) {
            command = command.args(["--topo-order", "--reverse"]);
        }
    }
    let output = command.args(revisions).arg("--").output().await?;
    let mut commits: Vec<String> = output.lines().map(str::to_owned).collect();
    if commits.is_empty() {
        bail!("cherry-pick selection is empty");
    }
    if let Some(count) = top {
        if commits.len() != count {
            bail!(
                "requested top {count} commits, but found only {}",
                commits.len()
            );
        }
        commits.reverse();
    }
    let merges = GitCommand::new(root, "rev-list")
        .args(["--no-walk", "--min-parents=2"])
        .args(&commits)
        .arg("--")
        .output()
        .await?;
    if !merges.is_empty() {
        bail!(
            "cannot cherry-pick merge commits without a mainline: {}",
            merges.trim()
        );
    }
    Ok(commits)
}

pub async fn cherry_pick(root: &Path, commit: &str) -> Result<()> {
    GitCommand::new(root, "cherry-pick")
        .arg("--no-commit")
        .arg(commit)
        .run()
        .await
}

pub async fn conflicted_files(root: &Path) -> Result<Vec<String>> {
    let output = GitCommand::new(root, "diff")
        .args(["--name-only", "--diff-filter=U", "-z"])
        .output()
        .await?;
    Ok(output.split_terminator('\0').map(str::to_owned).collect())
}

pub async fn stage_resolved_conflicts(root: &Path, files: &[String]) -> Result<bool> {
    for file in files {
        let content = match tokio::fs::read(root.join(file)).await {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => return Err(error).wrap_err_with(|| format!("failed to read {file}")),
        };
        if content.split(|byte| *byte == b'\n').any(|line| {
            [b"<<<<<<<", b"|||||||", b"=======", b">>>>>>>"]
                .iter()
                .any(|marker| line.starts_with(*marker))
        }) {
            eprintln!("Conflict markers remain in {file}.");
            return Ok(false);
        }
    }
    GitCommand::new(root, "add").arg("--update").run().await?;
    Ok(true)
}

pub async fn check_clean(root: &Path) -> Result<()> {
    let status = GitCommand::new(root, "status")
        .args(["--porcelain", "--untracked-files=no"])
        .output()
        .await?;
    if !status.is_empty() {
        bail!("commit or stash tracked changes before creating a game branch");
    }
    for state in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "sequencer",
        "rebase-merge",
        "rebase-apply",
    ] {
        let path = GitCommand::new(root, "rev-parse")
            .args(["--path-format=absolute", "--git-path", state])
            .output()
            .await?;
        if tokio::fs::try_exists(path.trim()).await? {
            bail!("finish the current Git operation before creating a game branch ({state})");
        }
    }
    Ok(())
}
