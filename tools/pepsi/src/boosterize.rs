use clap::Args;
use color_eyre::Result;

use argument_parsers::RobotAddress;
use repository::Repository;
use robot::Robot;

use crate::{gammaray::CommandExt, progress_indicator::ProgressIndicator};

#[derive(Args, Debug)]
pub struct Arguments {
    /// Robots to boosterize
    #[arg(required = true)]
    pub robots: Vec<RobotAddress>,
}

pub async fn boosterize(arguments: Arguments, repository: &Repository) -> Result<()> {
    let setup = &repository.root.join("tools/k1-setup");
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
                    .rsync_with_robot()?
                    .arg("--rsync-path=sudo rsync")
                    .arg("--info=progress2")
                    .arg(setup.join("containerd-booster.yaml"))
                    .arg(format!(
                        "{}:/opt/booster/config/containerd/config.yaml",
                        robot.address
                    ))
                    .rsync_with_log("uploading Booster service configuration", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl restart booster-daemon && sudo systemctl enable --now joystick_ros2")
                    .ssh_with_log("restoring Booster services", &progress_bar)
                    .await
            },
        )
        .await;

    Ok(())
}
