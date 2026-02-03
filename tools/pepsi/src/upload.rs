use std::path::Path;

use argument_parsers::RobotAddress;
use clap::Args;
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use repository::{upload::get_hulk_binary, Repository};
use robot::{Robot, SystemctlAction};
use tempfile::tempdir;

use crate::{
    cargo::{self, build, cargo, environment::EnvironmentArguments, CargoCommand},
    progress_indicator::{ProgressIndicator, Task},
};

#[derive(Args)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub upload: UploadArguments,
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct UploadArguments {
    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[arg(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[arg(long)]
    pub no_clean: bool,
    /// Skip the OS version check
    #[arg(long)]
    pub skip_os_check: bool,
    /// The Robots to upload to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

async fn upload_with_progress(
    robot_address: &RobotAddress,
    upload_directory: impl AsRef<Path>,
    arguments: &UploadArguments,
    progress: &Task,
    repository: &Repository,
) -> Result<()> {
    progress.set_message("Pinging Robot...");
    let robot = Robot::try_new_with_ping(robot_address.ip).await?;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let robot_os_version = robot
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {robot_address}"))?;
        let expected_os_version = repository
            .read_os_version()
            .await
            .wrap_err("failed to get configured OS version")?;
        if robot_os_version != expected_os_version {
            bail!(
                "mismatched OS versions: Expected {expected_os_version}, found {robot_os_version}"
            );
        }
    }

    progress.set_message("Stopping HULK...");
    robot
        .execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .wrap_err_with(|| format!("failed to stop HULK service on {robot_address}"))?;

    progress.set_message("Uploading: ...");
    robot
        .upload(upload_directory, "hulk", !arguments.no_clean, |status| {
            progress.set_message(format!("Uploading: {status}"))
        })
        .await
        .wrap_err_with(|| format!("failed to upload binary to {robot_address}"))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        if let Err(error) = robot
            .execute_systemctl(SystemctlAction::Start, "hulk")
            .await
        {
            let logs = robot
                .retrieve_logs()
                .await
                .wrap_err("failed to retrieve logs")?;
            bail!("failed to restart hulk: {error:#?}\nLogs:\n{logs}")
        };
    }
    Ok(())
}

pub async fn upload(arguments: Arguments, repository: &Repository) -> Result<()> {
    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    let hulk_binary = get_hulk_binary(arguments.build.profile());

    let cargo_arguments = cargo::Arguments {
        manifest: Some(
            repository
                .root
                .join("crates/hulk_booster/Cargo.toml")
                .into_os_string(),
        ),
        environment: arguments.environment,
        cargo: arguments.build,
    };

    if !arguments.upload.no_build {
        cargo(cargo_arguments, repository, &[&hulk_binary])
            .await
            .wrap_err("failed to build")?;
    }

    repository
        .populate_upload_directory(&upload_directory, hulk_binary)
        .await
        .wrap_err("failed to populate upload directory")?;

    let upload_arguments = &arguments.upload;
    let upload_directory = &upload_directory;

    let multi_progress = ProgressIndicator::new();
    arguments
        .upload
        .robots
        .iter()
        .map(|robot_address| {
            let progress = multi_progress.task(&robot_address.to_string());
            progress.enable_steady_tick();
            async move {
                progress.finish_with(
                    upload_with_progress(
                        robot_address,
                        upload_directory,
                        upload_arguments,
                        &progress,
                        repository,
                    )
                    .await
                    .as_ref(),
                )
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
