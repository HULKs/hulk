use std::{path::PathBuf, process::Stdio};

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::RobotAddress;
use repository::Repository;
use robot::Robot;
use tokio::io::AsyncWriteExt;

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Alternative path to an image
    #[arg(long)]
    image_path: Option<PathBuf>,
    /// Alternative HULKs-OS version e.g. 3.3
    #[arg(long)]
    version: Option<String>,
    /// The Robots to flash the image to, e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    robots: Vec<RobotAddress>,

    #[arg(short, long, default_value_t = {"123456".to_string()})]
    password: String,
}

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    // convert things to references to prevent moving into task closures
    let password = &arguments.password;
    let source_path = &repository.root.join("tools/k1-setup");

    ProgressIndicator::map_tasks(
        arguments.robots,
        "Sending gammaray to robot".to_string(),
        |robot_address, progress_bar| async move {
            let robot = Robot::try_new_with_ping(robot_address.ip).await?;

            progress_bar.set_message("Fixing sudo permissions");
            let mut child = robot
                .ssh_to_robot()?
                .arg("sudo true 2>/dev/null || sudo -S tee /etc/sudoers.d/booster")
                .stdin(Stdio::piped())
                .spawn()
                .wrap_err("failed to spawn ssh command")?;
            child
                .stdin
                .as_mut()
                .expect("child had no stdin")
                .write_all(
                    format!(
                        "{}\nbooster ALL=(ALL:ALL) NOPASSWD: ALL\nDefaults:booster verifypw=any\n",
                        password
                    )
                    .as_bytes(),
                )
                .await?;
            child
                .wait()
                .await
                .wrap_err("failed to fix sudo permissions")?;

            progress_bar.set_message("Uploading binaries");
            robot
                .rsync_with_robot()?
                .arg("--rsync-path=sudo rsync")
                .arg(source_path.join("hulk"))
                .arg(source_path.join("launchHULK"))
                .arg(format!("{}:/usr/bin/", robot.address))
                .status()
                .await
                .wrap_err("failed to upload binaries")?;

            progress_bar.set_message("Uploading service files");
            robot
                .rsync_with_robot()?
                .arg(source_path.join("hulk.service"))
                .arg(source_path.join("zenoh-bridge.service"))
                .arg(source_path.join("zenoh-bridge-ros2dds.service"))
                .arg(format!("{}:.config/systemd/user/", robot.address))
                .status()
                .await
                .wrap_err("failed to upload service files")?;

            robot
                .ssh_to_robot()?
                .arg("systemctl --user daemon-reload")
                .status()
                .await
                .wrap_err("failed to reload services")?;

            progress_bar.set_message("Installing zenoh-bridge-ros2dds");
            robot
                .ssh_to_robot()?
                .arg(INSTALL_ROS2DDS_ZENOH_BRIDGE)
                .status()
                .await?;
            robot
                .rsync_with_robot()?
                .arg(source_path.join("conf.json5"))
                .arg(format!("{}:/etc/zenoh-bridge-ros2dds/", robot.address))
                .status()
                .await?;

            // TODO: (re)start bridge services
            // TODO: do we need/want to reboot?
            // robot
            //     .reboot()
            //     .await
            //     .wrap_err_with(|| format!("failed to reboot {robot_address}"))
            Ok(())
        },
    )
    .await;

    Ok(())
}

static INSTALL_ROS2DDS_ZENOH_BRIDGE: &str = "
curl -L https://download.eclipse.org/zenoh/debian-repo/zenoh-public-key | sudo gpg --dearmor --yes --output /etc/apt/keyrings/zenoh-public-key.gpg
grep \"https://download.eclipse.org/zenoh/debian-repo/\" /etc/apt/sources.list || echo \"deb [signed-by=/etc/apt/keyrings/zenoh-public-key.gpg] https://download.eclipse.org/zenoh/debian-repo/ /\" | sudo tee -a /etc/apt/sources.list > /dev/null
sudo apt update
sudo apt install zenoh-bridge-ros2dds
";
