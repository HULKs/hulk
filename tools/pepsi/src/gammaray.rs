use std::{path::PathBuf, process::Stdio, str::FromStr, time::Duration};

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::RobotAddress;
use repository::Repository;
use robot::Robot;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::watch,
    time::sleep,
};

use crate::{
    cargo::{
        build::Arguments as CargoBuildArguments, construct_cargo_command,
        environment::EnvironmentArguments, Arguments as CargoArguments,
    },
    progress_indicator::ProgressIndicator,
};

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
    let setup = &repository.root.join("tools/k1-setup");

    let progress = ProgressIndicator::new();
    let (zenoh_bridge_status_sender, zenoh_bridge_status) = watch::channel(None);
    let zenoh_bridge_status_sender = Some(zenoh_bridge_status_sender);
    // let task = progress.task("Building zenoh bridge".to_owned());
    // zenoh_bridge_task.await;
    // return Ok(());
    progress
        .map_tasks(
            [RobotAddress::from_str("0").unwrap()]
                .into_iter()
                .chain(arguments.robots),
            "Sending gammaray to robot".to_string(),
            |robot_address, progress_bar| {
                if &robot_address.to_string() == "10.1.24.0" {
                let zenoh_bridge_task = tokio::spawn({
                    let repo: Repository = repository.clone();
    let zenoh_bridge_status_sender =zenoh_bridge_status_sender.take().unwrap();
                    async move {
                        // for i in (0..=10).rev() {
                        //     task.set_message(format!("{}", i));
                        //     sleep(Duration::from_secs(1)).await;
                        // }
                        let mut command = construct_cargo_command(
                            CargoArguments {
                                manifest: Some("crates/zenoh_bridge".into()),
                                environment: EnvironmentArguments {
                                    env: None,
                                    remote: false,
                                },
                                cargo: CargoBuildArguments::default(),
                            },
                            &repo,
                            &["target/aarch64-unknown-linux-gnu/debug/zenoh_bridge"],
                        )
                        .await
                        .expect("failed to construct cargo command");
                        command.stderr(Stdio::piped());
                        command.stdout(Stdio::piped());
                        let mut process = command.spawn().unwrap();
                        // let stderr = process.stderr.unwrap();
                        let mut stdout = BufReader::new(process.stdout.take().unwrap()).lines();
                        loop {
                            tokio::select! {
                                result = stdout.next_line() => {
                                    let Ok(Some(text)) = result else {break};
                                    progress_bar.set_message(format!("'{}'", &text[..100.min(text.len())]));
                                    // task.set_message(format!("'{}'", text.len()));
                                }
                            }
                        }
                        process.wait().await.unwrap();
                        zenoh_bridge_status_sender.send(Some(())).unwrap();
                        progress_bar.finish_with(Ok(()));
                    }
                });
                }
                let mut value = zenoh_bridge_status.clone();
                async move {
                    return Ok(());
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

                    progress_bar.set_message("Uploading service files");
                    robot
                        .rsync_with_robot()?
                        .arg(setup.join("hulk.service"))
                        .arg(setup.join("zenoh-bridge.service"))
                        .arg(setup.join("zenoh-bridge-ros2dds.service"))
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
                        .arg(setup.join("conf.json5"))
                        .arg(format!("{}:/etc/zenoh-bridge-ros2dds/", robot.address))
                        .status()
                        .await?;

                    progress_bar.set_message("Waiting for bridge to finish building");
                    value.wait_for(|value| value.is_some()).await.unwrap();

                    progress_bar.set_message("Uploading binaries");
                    robot
                        .rsync_with_robot()?
                        .arg("--rsync-path=sudo rsync")
                        .arg(setup.join("hulk"))
                        .arg(setup.join("launchHULK"))
                        .arg(
                            repository
                                .root
                                .join("target/aarch64-unknown-linux-gnu/debug/zenoh_bridge"),
                        )
                        .arg(format!("{}:/usr/bin/", robot.address))
                        .status()
                        .await
                        .wrap_err("failed to upload binaries")?;

                    // TODO: (re)start bridge services
                    // TODO: do we need/want to reboot?
                    // robot
                    //     .reboot()
                    //     .await
                    //     .wrap_err_with(|| format!("failed to reboot {robot_address}"))
                    Ok(())
                }
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
