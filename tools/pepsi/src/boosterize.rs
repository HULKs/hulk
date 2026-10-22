use clap::Args;
use color_eyre::Result;

use argument_parsers::RobotAddress;
use robot::Robot;

use crate::{
    gammaray::{CommandExt, MANUFACTURER_SERVICES_SCRIPT},
    progress_indicator::ProgressIndicator,
};

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
                crate::gammaray::overlay::disable(&robot, &progress_bar).await?;
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
                    .arg(format!(
                        "sudo bash -s -- enable /opt/booster/config/containerd/config.yaml <<'EOF'\n{}\nEOF",
                        MANUFACTURER_SERVICES_SCRIPT
                    ))
                    .ssh_with_log("restoring Booster services", &progress_bar)
                    .await
            },
        )
        .await;

    Ok(())
}
