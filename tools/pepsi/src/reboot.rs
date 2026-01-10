use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::RobotAddress;
use robot::Robot;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// The Robots to reboot e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn reboot(arguments: Arguments) -> Result<()> {
    ProgressIndicator::map_tasks(
        arguments.robots,
        "Rebooting...",
        |robot_address, _progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;
            robot
                .reboot()
                .await
                .wrap_err_with(|| format!("failed to reboot {robot_address}"))
        },
    )
    .await;

    Ok(())
}
