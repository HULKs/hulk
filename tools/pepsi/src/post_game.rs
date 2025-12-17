use std::{
    fmt::{Display, Formatter, Result as FormatResult},
    path::PathBuf,
};

use argument_parsers::RobotAddress;
use clap::{Args, ValueEnum};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};

use repository::Repository;
use robot::{Booster, Network, SystemctlAction};

use crate::{deploy_config::DeployConfig, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// Do not disconnect from the WiFi network
    #[arg(long)]
    pub no_disconnect: bool,
    /// Directory where to store the downloaded logs (will be created if not existing)
    #[arg(long)]
    pub log_directory: Option<PathBuf>,
    /// Current game phase
    #[arg(value_enum)]
    pub phase: Phase,
    /// The robots to apply the postgame to, queried from the deploy.toml if not specified
    pub robots: Option<Vec<RobotAddress>>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Phase {
    GoldenGoal,
    FirstHalf,
    SecondHalf,
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        match self {
            Phase::GoldenGoal => write!(f, "golden-goal"),
            Phase::FirstHalf => write!(f, "first-half"),
            Phase::SecondHalf => write!(f, "second-half"),
        }
    }
}

pub async fn post_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;

    let all_robots = config.all_robots();
    let robots = if let Some(robots) = &arguments.robots {
        for robot in robots {
            if !all_robots.contains(robot) {
                bail!("robot with IP {robot} is not specified in the deploy.toml");
            }
        }
        robots.iter().copied().collect()
    } else {
        all_robots
    };

    let log_directory = &arguments.log_directory.unwrap_or_else(|| {
        let log_directory_name = config.log_directory_name();

        repository.root.join("logs").join(log_directory_name)
    });

    ProgressIndicator::map_tasks(
        robots,
        "Executing postgame tasks...",
        |robot_address, progress_bar| async move {
            progress_bar.set_message("Pinging Robot...");
            let robot = Booster::ping_until_available(robot_address.ip).await;

            progress_bar.set_message("Stopping HULK service...");
            robot
                .execute_systemctl(SystemctlAction::Stop, "hulk")
                .await
                .wrap_err_with(|| format!("failed to execute systemctl hulk on {robot_address}"))?;

            if !arguments.no_disconnect {
                progress_bar.set_message("Disconnecting from WiFi...");
                robot
                    .set_wifi(Network::None)
                    .await
                    .wrap_err_with(|| format!("failed to set network on {robot_address}"))?;
            }

            progress_bar.set_message("Downloading logs...");
            let log_directory = log_directory
                .join(arguments.phase.to_string())
                .join(robot_address.to_string());
            robot
                .download_logs(log_directory, |status| {
                    progress_bar.set_message(format!("Downloading logs: {status}"))
                })
                .await
                .wrap_err_with(|| format!("failed to download logs from {robot_address}"))?;

            Ok(())
        },
    )
    .await;

    Ok(())
}
