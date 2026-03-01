use std::{path::Path, process::Stdio};

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
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
    sync::watch,
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
    /// The Robots to flash the image to, e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    robots: Vec<RobotAddress>,

    // The password for the `booster` user
    #[arg(short, long, default_value_t = {"123456".to_string()})]
    password: String,
}

static ADD_APT_ROS2DDS_ZENOH_BRIDGE_SOURCES: &str = "
curl -L https://download.eclipse.org/zenoh/debian-repo/zenoh-public-key | sudo gpg --dearmor --yes --output /etc/apt/keyrings/zenoh-public-key.gpg
grep \"https://download.eclipse.org/zenoh/debian-repo/\" /etc/apt/sources.list || echo \"deb [signed-by=/etc/apt/keyrings/zenoh-public-key.gpg] https://download.eclipse.org/zenoh/debian-repo/ /\" | sudo tee -a /etc/apt/sources.list > /dev/null
";

static PACKAGES: [&str; 2] = ["zenoh-bridge-ros2dds", "podman"];

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    let setup_path = &repository.root.join("tools/k1-setup");

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
                    &arguments.password,
                    repository,
                    setup_path,
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
    password: &str,
    repository: &Repository,
    setup: &Path,
    mut zenoh_bridge_status: watch::Receiver<Option<bool>>,
) -> Result<()> {
    let robot = Robot::try_new_with_ping(robot.ip).await?;

    robot
        .ssh_to_robot()?
        .arg(format!(
            // `sudo true` only succeeds if passwordless sudo is already allowed.
            // In that case the `||` skips the remainder of the command to make it idempotent.
            // This is necessary because the two lines for the sudoers file and the password are
            // all in the same stream. If passwordless sudo is already enabled, `sudo -S` does not
            // consume the first line, causing the password to be written to the sudoers file as
            // well.
            r#"sudo true 2>/dev/null || printf '{}\nbooster ALL=(ALL:ALL) NOPASSWD: ALL\nDefaults:booster verifypw=any\n' | sudo -S tee /etc/sudoers.d/booster"#,
            password
        ))
        .ssh_with_log("enabling passwordless sudo", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg(ADD_APT_ROS2DDS_ZENOH_BRIDGE_SOURCES)
        .ssh_with_log("adding zenoh-bridge-ros2dds sources", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo apt update && sudo apt install")
        .args(PACKAGES)
        .ssh_with_log("installing packages", &progress_bar)
        .await?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg(setup.join("conf.json5"))
        .arg(format!("{}:/etc/zenoh-bridge-ros2dds/", robot.address))
        .rsync_with_log("uploading zenoh-bridge-ros2dds config", &progress_bar)
        .await?;

    robot
        .rsync_with_robot()?
        .arg(setup.join("hulk.service"))
        .arg(setup.join("zenoh-bridge.service"))
        .arg(setup.join("zenoh-bridge-ros2dds.service"))
        .arg(format!("{}:.config/systemd/user/", robot.address))
        .rsync_with_log("uploading service files", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("mkdir -p /home/booster/.cache/hulk/tensor-rt/")
        .ssh_with_log("creating tensorrt cache directory", &progress_bar)
        .await?;

    progress_bar.set_message("Waiting for zenoh bridge to finish building");
    if !zenoh_bridge_status
        .wait_for(|value| value.is_some())
        .await?
        .unwrap()
    {
        bail!("building bridge failed, aborting");
    }

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg(setup.join("hulk"))
        .arg(setup.join("launchHULK"))
        .arg(
            repository
                .root
                .join("target/aarch64-unknown-linux-gnu/release/zenoh_bridge"),
        )
        .arg(format!("{}:/usr/bin/", robot.address))
        .rsync_with_log("uploading binaries", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("systemctl --user daemon-reload")
        .ssh_with_log("reloading service daemon", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("systemctl --user enable --now")
        .args(["hulk", "zenoh-bridge", "zenoh-bridge-ros2dds"])
        .ssh_with_log("restarting services", &progress_bar)
        .await?;

    Ok(())
}

trait CommandExt {
    async fn ssh_with_log(&mut self, prefix: &str, progress_bar: &ProgressBar) -> Result<()>;

    async fn rsync_with_log(&mut self, name: &str, progress_bar: &ProgressBar) -> Result<()>;

    async fn run_with_log(
        &mut self,
        name: &str,
        progress_bar: &ProgressBar,
        line_delimiter: u8,
    ) -> Result<()>;
}

impl CommandExt for Command {
    async fn ssh_with_log(&mut self, name: &str, progress_bar: &ProgressBar) -> Result<()> {
        self.run_with_log(name, progress_bar, b'\n').await
    }

    async fn rsync_with_log(&mut self, name: &str, progress_bar: &ProgressBar) -> Result<()> {
        self.run_with_log(name, progress_bar, b'\r').await
    }

    async fn run_with_log(
        &mut self,
        name: &str,
        progress_bar: &ProgressBar,
        line_delimiter: u8,
    ) -> Result<()> {
        progress_bar.set_message(name.to_string());
        self.stdout(Stdio::piped());
        self.stderr(Stdio::piped());
        let mut process = self.spawn().unwrap();
        let mut lines = BufReader::new(process.stdout.take().unwrap()).split(line_delimiter);

        while let Ok(Some(buffer)) = lines.next_segment().await {
            if let Ok(text) = std::str::from_utf8(&buffer) {
                progress_bar.set_message(format!("{name}: {text}"));
            }
        }

        let maybe_code = process
            .wait()
            .await
            .wrap_err_with(|| format!("failed at {name}"))?
            .code();
        match maybe_code {
            Some(0) => Ok(()),
            None => bail!("process was killed"),
            Some(code) => {
                let mut stderr = String::new();
                process
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut stderr)
                    .await?;
                Err(eyre!("process exited with error code {code}\n{stderr}"))
            }
        }
    }
}

async fn build_bridge(
    repository: Repository,
    zenoh_bridge_status_sender: watch::Sender<Option<bool>>,
    progress_bar: Task,
) -> Result<()> {
    let mut command = construct_cargo_command(
        CargoArguments {
            manifest: Some("crates/zenoh_bridge".into()),
            environment: EnvironmentArguments {
                env: None,
                remote: false,
            },
            cargo: CargoBuildArguments {
                release: true,
                ..Default::default()
            },
        },
        &repository,
        &["target/aarch64-unknown-linux-gnu/release/zenoh_bridge"],
    )
    .await
    .expect("failed to construct cargo command");

    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command.stdin(Stdio::null());
    command.kill_on_drop(true);

    let mut process = command.spawn().unwrap();

    process.stdin.take();
    process.stderr.take();
    let mut lines = BufReader::new(process.stdout.take().unwrap()).lines();
    while let Ok(Some(text)) = lines.next_line().await {
        progress_bar.progress.println(text);
    }
    let status = process.wait().await.unwrap();
    if !status.success() {
        zenoh_bridge_status_sender.send(Some(false)).unwrap();
        progress_bar.finish_with_error(eyre!("failed with code {}", status.code().unwrap()));
        bail!("process failed");
    }
    progress_bar.finish_with_success(());
    zenoh_bridge_status_sender.send(Some(true)).unwrap();

    Ok(())
}
