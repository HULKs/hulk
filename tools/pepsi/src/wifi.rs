use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Subcommand,
};
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::{parse_network, RobotAddress, NETWORK_POSSIBLE_VALUES};
use robot::{Network, Robot};

use crate::progress_indicator::ProgressIndicator;

#[derive(Subcommand)]
pub enum Arguments {
    /// List available networks
    List {
        /// The Robots to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// Scan for networks
    Scan {
        /// The Robots to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// Set active network
    Set {
        /// The network to connect the wireless device to (None disconnects from anything)
        #[arg(
            value_parser = PossibleValuesParser::new(NETWORK_POSSIBLE_VALUES)
                .map(|s| parse_network(&s).unwrap()))
        ]
        network: Network,
        /// The Robots to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
    /// Show current network status
    Status {
        /// The Robots to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        robots: Vec<RobotAddress>,
    },
}

pub async fn wifi(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::Status { robots } => status(robots).await,
        Arguments::Scan { robots } => scan(robots).await,
        Arguments::List { robots } => available_networks(robots).await,
        Arguments::Set { network, robots } => set(robots, network).await,
    };

    Ok(())
}

async fn status(robots: Vec<RobotAddress>) {
    ProgressIndicator::map_tasks(
        robots,
        "Retrieving network status...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .get_network_status()
                .await
                .wrap_err_with(|| format!("failed to get network status from {robot_address}"))
        },
    )
    .await;
}

async fn scan(robots: Vec<RobotAddress>) {
    ProgressIndicator::map_tasks(
        robots,
        "Starting network scan...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .scan_networks()
                .await
                .wrap_err_with(|| format!("failed to scan for networks on {robot_address}"))
        },
    )
    .await;
}

async fn available_networks(robots: Vec<RobotAddress>) {
    ProgressIndicator::map_tasks(
        robots,
        "Retrieving available networks...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .get_available_networks()
                .await
                .wrap_err_with(|| format!("failed to get available networks from {robot_address}"))
        },
    )
    .await;
}

async fn set(robots: Vec<RobotAddress>, network: Network) {
    ProgressIndicator::map_tasks(
        robots,
        "Setting network...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .set_wifi(network)
                .await
                .wrap_err_with(|| format!("failed to set network on {robot_address}"))
        },
    )
    .await;
}
