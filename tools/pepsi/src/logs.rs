use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::RobotAddress;
use robot::Booster;

use crate::progress_indicator::ProgressIndicator;

#[derive(Subcommand)]
pub enum Arguments {
    /// Delete logs on the Robots
    Delete {
        /// The Robots to delete logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// Download logs from the Robots
    Download {
        /// Directory where to store the downloaded logs (will be created if not existing)
        log_directory: PathBuf,
        /// The Robots to download logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// List logs from Robots
    List {
        /// The Robot to show logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// Show logs from Robots
    Show {
        /// The Robot to show logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
}

pub async fn logs(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::Delete { robots } => {
            ProgressIndicator::map_tasks(
                robots,
                "Deleting logs...",
                |robot_address, _progress_bar| async move {
                    let robot = Booster::try_new_with_ping(robot_address.ip).await?;
                    robot
                        .delete_logs()
                        .await
                        .wrap_err_with(|| format!("failed to delete logs on {robot_address}"))
                },
            )
            .await
        }
        Arguments::Download {
            log_directory,
            robots,
        } => {
            ProgressIndicator::map_tasks(
                robots,
                "Downloading logs: ...",
                |robot_address, progress| {
                    let log_directory = log_directory.join(robot_address.to_string());
                    async move {
                        let robot = Booster::try_new_with_ping(robot_address.ip).await?;
                        robot
                            .download_logs(log_directory, |status| {
                                progress.set_message(format!("Downloading logs: {status}"))
                            })
                            .await
                            .wrap_err_with(|| {
                                format!("failed to download logs from {robot_address}")
                            })
                    }
                },
            )
            .await
        }
        Arguments::List { robots } => {
            ProgressIndicator::map_tasks(
                robots,
                "Retrieving all logs...",
                |robot_address, _progress_bar| async move {
                    let robot = Booster::try_new_with_ping(robot_address.ip).await?;
                    robot.list_logs().await.wrap_err("failed to retrieve logs")
                },
            )
            .await
        }
        Arguments::Show { robots } => {
            ProgressIndicator::map_tasks(
                robots,
                "Retrieving latest logs...",
                |robot_address, _progress_bar| async move {
                    let robot = Booster::try_new_with_ping(robot_address.ip).await?;
                    robot
                        .retrieve_logs()
                        .await
                        .wrap_err("failed to retrieve logs")
                },
            )
            .await
        }
    }

    Ok(())
}
