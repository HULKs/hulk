use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::{number_to_ip, Connection, RobotAddress};
use futures_util::{stream::FuturesUnordered, StreamExt};
use repository::Repository;
use robot::Robot;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Power off all Robots
    #[arg(long)]
    pub all: bool,
    /// The Robots to power off e.g. 20w or 10.1.24.22
    #[arg(required = true, conflicts_with = "all", num_args = 1..)]
    pub robots: Vec<RobotAddress>,
}

pub async fn power_off(arguments: Arguments, repository: &Repository) -> Result<()> {
    if arguments.all {
        let team = repository
            .read_team_configuration()
            .await
            .wrap_err("failed to get team configuration")?;
        let addresses = team
            .robots
            .iter()
            .map(|robot| async move {
                let host = number_to_ip(robot.number, Connection::Wired)?;
                match Robot::try_new_with_ping(host).await {
                    Ok(robot) => Ok(robot),
                    Err(_) => {
                        let host = number_to_ip(robot.number, Connection::Wireless)?;
                        Robot::try_new_with_ping(host).await
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        ProgressIndicator::new()
            .map_tasks(
                addresses.into_iter().filter_map(|robot| robot.ok()),
                "Powering off...",
                |robot, _progress_bar| async move {
                    robot
                        .power_off()
                        .await
                        .wrap_err_with(|| format!("failed to power {robot} off"))
                },
            )
            .await;
    } else {
        ProgressIndicator::new()
            .map_tasks(
                arguments.robots,
                "Powering off...",
                |robot_address, _progress_bar| async move {
                    let robot = Robot::try_new_with_ping(robot_address.ip).await?;
                    robot
                        .power_off()
                        .await
                        .wrap_err_with(|| format!("failed to power {robot_address} off"))
                },
            )
            .await;
    }
    Ok(())
}
