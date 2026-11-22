use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use repository::{Repository, sdk};

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
    if arguments.environment.remote {
        bail!("prepare the SDK on the remote host, or let a remote build prepare it automatically");
    }
    if matches!(arguments.environment.env, Some(Environment::Native)) {
        bail!("SDK preparation requires the robot environment (--env nix)");
    }
    let status = sdk::robot_command(&repository.root)
        .arg("true")
        .current_dir(&repository.root)
        .status()
        .await
        .wrap_err("failed to prepare robot SDK")?;
    if !status.success() {
        bail!("SDK preparation failed with {status}");
    }
    Ok(())
}
