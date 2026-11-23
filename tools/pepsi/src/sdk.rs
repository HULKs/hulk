use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use repository::{Repository, sdk};
use tokio::process::Command;

use crate::cargo::environment::{Environment, EnvironmentArguments};

#[derive(Subcommand)]
pub enum Arguments {
    /// Download or build the pinned robot SDK (without compiling robot code)
    #[command(visible_aliases = ["install", "installier", "build", "bau"])]
    Prepare(PrepareArguments),
}

#[derive(Args)]
pub struct PrepareArguments {
    #[command(flatten)]
    pub environment: EnvironmentArguments,
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    let Arguments::Prepare(arguments) = arguments;
    if matches!(arguments.environment.env, Some(Environment::Native)) {
        bail!("SDK preparation requires the robot environment (--env nix)");
    }
    let mut command = if arguments.environment.remote {
        let mut command = Command::new(repository.root.join("scripts/remote_workspace"));
        command.args([
            "--cd",
            ".",
            "bash",
            "tools/run-nix-shell",
            ".",
            "robot-sdk",
            "true",
        ]);
        command
    } else {
        let mut command = sdk::robot_command(&repository.root);
        command.arg("true");
        command
    };
    let status = command
        .current_dir(&repository.root)
        .status()
        .await
        .wrap_err("failed to prepare robot SDK")?;
    if !status.success() {
        bail!("SDK preparation failed with {status}");
    }
    Ok(())
}
