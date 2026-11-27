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

    let root = &repository.root;
    let mut command = if arguments.environment.remote {
        let remote_root = Path::new(".");
        let build = develop(remote_root, Environment::NixRobot, true, "true".as_ref());
        remote_command(root, remote_root, &build, &[] as &[&Path])
    } else {
        develop(root, Environment::NixRobot, false, "true".as_ref())
    };

    command.current_dir(root);
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
    Ok(remote_command(
        &repository.root,
        &repository.root_to_current_dir()?,
        &build,
        artifacts,
    ))
}

fn remote_command(
    root: &Path,
    working_directory: &Path,
    build: &Command,
    artifacts: &[impl AsRef<Path>],
) -> Command {
    let mut command = Command::new(root.join("scripts/remote_workspace"));
    for artifact in artifacts {
        command.arg("--return-file").arg(artifact.as_ref());
    }
    command.arg("--cd").arg(working_directory);
    command.arg(build.as_std().get_program());
    command.args(build.as_std().get_args());
    command
}

fn develop(root: &Path, environment: Environment, remote: bool, program: &OsStr) -> Command {
    let shell = match environment {
        Environment::PathCargo => return Command::new(program),
        Environment::NixNative if !remote && in_native_shell() => return Command::new(program),
        Environment::NixNative => "default",
        Environment::NixRobot => "robot-sdk",
    };
    let mut command = Command::new("bash");
    command
        .arg(root.join("scripts/nix_shell"))
        .arg(root)
        .arg(shell);
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

/// Run a command and report failures with the operation name.
pub async fn run(command: &mut Command, operation: &str) -> Result<()> {
    let status = command.status().await.wrap_err_with(|| {
        let program = command.as_std().get_program().to_string_lossy();
        format!("failed to {operation}: could not launch {program}")
    })?;
    ensure!(status.success(), "failed to {operation}: {status}");
    Ok(())
}
