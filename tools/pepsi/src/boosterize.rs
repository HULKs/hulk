use clap::Args;
use color_eyre::Result;

use argument_parsers::RobotAddress;
use robot::Robot;

use crate::{gammaray::CommandExt, progress_indicator::ProgressIndicator};

#[derive(Args, Debug)]
pub struct Arguments {
    /// Robots to boosterize
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn boosterize(arguments: Arguments) -> Result<()> {
    let progress = ProgressIndicator::new();

    progress
        .map_tasks(
            arguments.robots,
            "Boosterizing robot".to_string(),
            |robot, progress_bar| async move {
                let robot = Robot::try_new_with_ping(robot.ip).await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl disable --now")
                    .arg("hulk-runtime")
                    .ssh_with_log("disabling hulk-runtime", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl disable --now")
                    .arg("hulk")
                    .ssh_with_log("disabling hulk", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl enable --now")
                    .arg("booster-daemon-perception")
                    .ssh_with_log("enabling booster-daemon-perception", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl enable --now")
                    .arg("booster-agent-manager")
                    .ssh_with_log("enabling booster-agent-manager", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl enable --now")
                    .arg("booster-lui")
                    .ssh_with_log("enabling booster-lui", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl enable --now")
                    .arg("booster-rtc-speech")
                    .ssh_with_log("enabling booster-rtc-speech", &progress_bar)
                    .await
            },
        )
        .await;

    Ok(())
}
