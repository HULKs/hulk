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
                    .arg(setup.join("child-booster.ini"))
                    .arg(format!("{}:/opt/booster/Daemon/bin/child.ini", robot.address))
                    .rsync_with_log("uploading Booster controller configuration", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg(concat!(
                        "sudo rm -f /etc/udev/rules.d/99-hulk-microphone.rules && ",
                        "sudo udevadm control --reload-rules && ",
                        "sudo udevadm trigger --action=change --subsystem-match=sound --sysname-match='card*' && ",
                        "sudo udevadm settle --timeout=10 && ",
                        "XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user try-restart pulseaudio.service",
                    ))
                    .ssh_with_log("restoring PulseAudio microphone access", &progress_bar)
                    .await?;
                robot
                    .ssh_to_robot()?
                    .arg("sudo systemctl restart booster-daemon && sudo systemctl enable --now joystick_ros2")
                    .ssh_with_log("restoring Booster controller", &progress_bar)
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
