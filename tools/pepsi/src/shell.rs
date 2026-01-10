use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use robot::Robot;

use argument_parsers::RobotAddress;

#[derive(Args)]
pub struct Arguments {
    /// The Robot to connect to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub robot: RobotAddress,
}

pub async fn shell(arguments: Arguments) -> Result<()> {
    let robot = Robot::try_new_with_ping(arguments.robot.ip).await?;

    robot
        .execute_shell()
        .await
        .wrap_err_with(|| format!("failed to execute shell on {}", arguments.robot))
}
