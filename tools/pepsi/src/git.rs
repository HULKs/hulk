use std::ffi::OsStr;

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

    async fn run(mut self) -> Result<()> {
        let status = self.inner.status().await.wrap_err("failed to run git")?;

        if !status.success() {
            bail!("git failed with {status}");
        }

        Ok(())
    }
}

pub async fn create_and_switch_to_branch(name: &str, base: &str) -> Result<()> {
    GitCommand::new("switch")
        .arg("-c")
        .arg(name)
        .arg(base)
        .run()
        .await
}

pub async fn switch_to_branch(name: &str) -> Result<()> {
    GitCommand::new("switch").arg(name).run().await
}

pub async fn create_commit(message: &str) -> Result<()> {
    GitCommand::new("commit").arg("-m").arg(message).run().await
}

pub async fn reset_to_head() -> Result<()> {
    GitCommand::new("reset")
        .arg("--hard")
        .arg("HEAD")
        .run()
        .await
}
