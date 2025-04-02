use std::{ffi::OsStr, process::ExitStatus};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::process::Command;

pub struct GitCommand {
    inner: Command,
}

impl GitCommand {
    fn new(subcommand: impl AsRef<OsStr>) -> Self {
        let mut inner = Command::new("git");
        inner.arg(subcommand);

        Self { inner }
    }

    fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.inner.arg(arg);
        self
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

pub async fn create_and_switch_to_branch(name: &str, base: &str, force: bool) -> Result<()> {
    let create_flag = if force { "--force-create" } else { "--create" };

    GitCommand::new("switch")
        .arg(create_flag)
        .arg(name)
        .arg(base)
        .run()
        .await
}

pub async fn create_commit(message: &str) -> Result<()> {
    GitCommand::new("commit")
        .arg("--all")
        .arg("--allow-empty")
        .arg("--message")
        .arg(message)
        .run()
        .await
}

pub async fn reset_to_head() -> Result<()> {
    GitCommand::new("reset")
        .arg("--hard")
        .arg("HEAD")
        .run()
        .await
}
