use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::RobotAddress;
use indicatif::ProgressBar;
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
    progress_indicator::{ProgressIndicator, Task},
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
    let setup: &PathBuf = &repository.root.join("tools/k1-setup");

    let progress = ProgressIndicator::new();
    let (zenoh_bridge_status_sender, zenoh_bridge_status) = watch::channel(None);
    let zenoh_bridge_build_task = tokio::spawn(build_bridge(
        repository.clone(),
        zenoh_bridge_status_sender,
        progress.task("Building zenoh bridge".to_owned()),
    ));
    progress
        .map_tasks(
            arguments.robots,
            "Sending gammaray to robot".to_string(),
            |robot, progress_bar| {
                gammaray_robot(
                    robot,
                    progress_bar,
                    password.to_string(),
                    repository,
                    setup,
                    zenoh_bridge_status.clone(),
                )
            },
        )
        .await;
    zenoh_bridge_build_task.await??;

    Ok(())
}

async fn gammaray_robot(
    robot: RobotAddress,
    progress_bar: ProgressBar,
    password: String,
    repository: &Repository,
    setup: &Path,
    mut zenoh_bridge_status: watch::Receiver<Option<()>>,
) -> Result<()> {
    progress_bar.set_message("working");
    sleep(Duration::from_secs(1)).await;
    progress_bar.set_message("Waiting for bridge to finish building");
    zenoh_bridge_status
        .wait_for(|value| value.is_some())
        .await
        .unwrap();
    progress_bar.set_message("bridge done");

    let robot = Robot::try_new_with_ping(robot.ip).await?;

    progress_bar.set_message("Fixing sudo permissions");
    fix_sudo_permissions(password, &robot).await?;

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
    zenoh_bridge_status
        .wait_for(|value| value.is_some())
        .await
        .unwrap();

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

async fn fix_sudo_permissions(
    password: String,
    robot: &Robot,
) -> Result<(), color_eyre::eyre::Error> {
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
    if !child
        .wait()
        .await
        .wrap_err("failed to fix sudo permissions")?
        .success()
    {
        bail!("failed to fix sudo permissions")
    }

    Ok(())
}

async fn build_bridge(
    repository: Repository,
    zenoh_bridge_status_sender: watch::Sender<Option<()>>,
    progress_bar: Task,
) -> Result<()> {
    let mut command = construct_cargo_command(
        CargoArguments {
            manifest: Some("crates/zenoh_bridge".into()),
            environment: EnvironmentArguments {
                env: None,
                remote: false,
            },
            cargo: CargoBuildArguments::default(),
        },
        &repository,
        &["target/aarch64-unknown-linux-gnu/debug/zenoh_bridge"],
    )
    .await
    .expect("failed to construct cargo command");

    dbg!(&command);

    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command.stdin(Stdio::null());
    command.kill_on_drop(true);

    let mut process = command.spawn().unwrap();

    let mut lines = BufReader::new(process.stdout.take().unwrap()).lines();
    while let Ok(Some(text)) = lines.next_line().await {
        progress_bar.progress.println(text);
    }
    let status = process.wait().await.unwrap();
    if !status.success() {
        progress_bar.finish_with_error(eyre!("failed with code {}", status.code().unwrap()));
    }
    progress_bar.finish_with_success(());
    zenoh_bridge_status_sender.send(Some(())).unwrap();

    Ok(())
}

static INSTALL_ROS2DDS_ZENOH_BRIDGE: &str = "
curl -L https://download.eclipse.org/zenoh/debian-repo/zenoh-public-key | sudo gpg --dearmor --yes --output /etc/apt/keyrings/zenoh-public-key.gpg
grep \"https://download.eclipse.org/zenoh/debian-repo/\" /etc/apt/sources.list || echo \"deb [signed-by=/etc/apt/keyrings/zenoh-public-key.gpg] https://download.eclipse.org/zenoh/debian-repo/ /\" | sudo tee -a /etc/apt/sources.list > /dev/null
sudo apt update
sudo apt install zenoh-bridge-ros2dds
";
