use std::{env, ffi::OsStr, path::Path};

use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail, ensure},
};
use repository::Repository;
use tokio::process::Command;

use crate::cargo::environment::{Environment, EnvironmentArguments};

/// Default repository-relative Cargo output directory for robot builds.
pub const ROBOT_TARGET_DIRECTORY: &str = "target/robot";

/// Robot SDK subcommands.
#[derive(Subcommand)]
pub enum Arguments {
    /// Download or build the pinned robot SDK (without compiling robot code)
    #[command(visible_aliases = ["install", "installier", "build", "bau"])]
    Prepare(PrepareArguments),
}

/// Options for preparing the robot SDK.
#[derive(Args)]
pub struct PrepareArguments {
    #[command(flatten)]
    environment: EnvironmentArguments,
}

/// Run the requested robot SDK subcommand.
pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    let Arguments::Prepare(arguments) = arguments;
    if !matches!(
        arguments.environment.env,
        None | Some(Environment::NixRobot)
    ) {
        bail!("SDK preparation requires the robot environment (--env nix-robot)");
    }
    let mut command = build_command(
        repository,
        Environment::NixRobot,
        arguments.environment.remote,
        "true",
        &[] as &[&Path],
    )?;
    run(&mut command, "prepare robot SDK").await
}

/// Check whether the current environment is the workstation Nix shell.
pub fn in_native_shell() -> bool {
    env::var("HULK_NATIVE_SHELL").as_deref() == Ok("1")
}

/// Construct a command that enters the workstation Nix shell if needed.
pub fn native_command(root: &Path, program: impl AsRef<OsStr>) -> Command {
    develop(root, Environment::NixNative, false, program.as_ref())
}

/// Construct a local or remote build command and request any remote artifacts.
pub fn build_command(
    repository: &Repository,
    environment: Environment,
    remote: bool,
    program: &str,
    artifacts: &[impl AsRef<Path>],
) -> Result<Command> {
    if !remote {
        return Ok(develop(
            &repository.root,
            environment,
            false,
            program.as_ref(),
        ));
    }

    let build = develop(
        &repository.current_dir_to_root()?,
        environment,
        true,
        program.as_ref(),
    );
    let mut command = Command::new(repository.root.join("scripts/remote_workspace"));
    for artifact in artifacts {
        command.arg("--return-file").arg(artifact.as_ref());
    }
    command.arg("--cd").arg(repository.root_to_current_dir()?);
    command.arg(build.as_std().get_program());
    command.args(build.as_std().get_args());
    Ok(command)
}

fn develop(root: &Path, environment: Environment, remote: bool, program: &OsStr) -> Command {
    let shell = match environment {
        Environment::PathCargo => return Command::new(program),
        Environment::NixNative if !remote && in_native_shell() => return Command::new(program),
        Environment::NixNative => "default",
        Environment::NixRobot => "robot-sdk",
    };
    let mut command = if remote {
        let mut command = Command::new("bash");
        command
            .arg(root.join("scripts/remote_nix_shell"))
            .arg(root)
            .arg(shell);
        command
    } else {
        let mut installable = root.as_os_str().to_os_string();
        installable.push(format!("#{shell}"));
        let mut command = nix();
        command.arg("develop").arg(installable);
        command
    };
    if environment == Environment::NixRobot {
        command
            .args(["--set-env-var", "HULK_ROBOT_BUILD_ROOT"])
            .arg(root);
    }
    command
        .args(["--no-update-lock-file", "--command"])
        .arg(program);
    command
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
