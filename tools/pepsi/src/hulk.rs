use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::{parse_systemctl_action, RobotAddress, SYSTEMCTL_ACTION_POSSIBLE_VALUES};
use robot::{Robot, SystemctlAction};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The systemctl action to execute for the HULK service
    #[arg(
        value_parser = PossibleValuesParser::new(SYSTEMCTL_ACTION_POSSIBLE_VALUES)
            .map(|s| parse_systemctl_action(&s).unwrap()))
    ]
    pub action: SystemctlAction,
    /// The Robots to execute that command on e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn hulk(arguments: Arguments) -> Result<()> {
    ProgressIndicator::map_tasks(
        arguments.robots,
        "Executing systemctl hulk...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .execute_systemctl(arguments.action, "hulk")
                .await
                .wrap_err_with(|| format!("failed to execute systemctl hulk on {robot_address}"))
        },
    )
    .await;

    Ok(())
}
