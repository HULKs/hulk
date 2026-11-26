use std::path::Path;

use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail, ensure},
};
use repository::{Repository, sdk};
use tokio::process::Command;

use crate::cargo::environment::{Environment, EnvironmentArguments};

/// Default repository-relative Cargo output directory for robot builds.
pub const ROBOT_TARGET_DIRECTORY: &str = "target/robot";

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
    if !matches!(
        arguments.environment.env,
        None | Some(Environment::NixRobot)
    ) {
        bail!("SDK preparation requires the robot environment (--env nix-robot)");
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

/// Construct a local or remote build command and request any remote artifacts.
pub fn build_command(
    repository: &Repository,
    environment: Environment,
    remote: bool,
    program: &str,
    artifacts: &[impl AsRef<Path>],
) -> Result<Command> {
    let root = if remote {
        repository.current_dir_to_root()?
    } else {
        repository.root.clone()
    };
    let command = match environment {
        Environment::PathCargo => Command::new(program),
        Environment::NixNative => {
            let mut command = sdk::native_command(&root);
            command.arg(program);
            command
        }
        Environment::NixRobot => {
            let mut command = sdk::robot_command(&root);
            command.arg(program);
            command
        }
    };
    if !remote {
        return Ok(command);
    }

    let mut remote_command = Command::new(repository.root.join("scripts/remote_workspace"));
    for artifact in artifacts {
        remote_command.arg("--return-file").arg(artifact.as_ref());
    }
    remote_command
        .arg("--cd")
        .arg(repository.root_to_current_dir()?);
    remote_command.arg(command.as_std().get_program());
    remote_command.args(command.as_std().get_args());
    Ok(remote_command)
}

/// Construct a Nix command with the flakes CLI features enabled.
pub fn nix() -> Command {
    let mut command = Command::new("nix");
    command.args(["--extra-experimental-features", "nix-command flakes"]);
    command
}

/// Run a command and report launch or exit failures with the operation name.
pub async fn run(command: &mut Command, operation: &str) -> Result<()> {
    let status = command
        .status()
        .await
        .wrap_err_with(|| format!("failed to {operation}"))?;
    ensure!(status.success(), "failed to {operation}: {status}");
    Ok(())
}
