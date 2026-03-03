use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};

use argument_parsers::RobotAddress;
use indicatif::ProgressBar;
use repository::{team::Team, Repository};
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

    /// The password for the `booster` user
    #[arg(short, long, default_value = "123456")]
    password: String,

    /// Optional podman image for the hulk service environment
    /// e.g. rust-trt-inference-image.tar
    #[arg(short, long)]
    image_file: Option<PathBuf>,
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

    let team = repository.read_team_configuration().await?;

    progress
        .map_tasks(
            arguments.robots,
            "Sending gammaray to robot".to_string(),
            |robot, progress_bar| {
                gammaray_robot(
                    robot,
                    progress_bar,
                    &arguments.password,
                    arguments.image_file.as_deref(),
                    repository,
                    &team,
                    setup_path,
                    zenoh_bridge_status.clone(),
                )
            },
        )
        .await;

    zenoh_bridge_build_task.await??;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn gammaray_robot(
    robot: RobotAddress,
    progress_bar: ProgressBar,
    password: &str,
    image_file: Option<&Path>,
    repository: &Repository,
    team: &Team,
    setup: &Path,
    mut zenoh_bridge_status: watch::Receiver<Option<bool>>,
) -> Result<()> {
    let robot = Robot::try_new_with_ping(robot.ip).await?;

    progress_bar.set_message("getting robot ID");
    let output = robot
        .ssh_to_robot()?
        // jetson_release always outputs control chars to color the left side.
        // The first grep gets rid of unwanted lines, the second matches only
        // the digits of the serial number, ignoring the control characters.
        .arg("jetson_release -s | grep 'Serial Number:' | grep '[0-9]*$' -o")
        .output()
        .await?;
    let id = String::from_utf8(output.stdout).unwrap();
    let id = id.trim();
    let Some(team_robot) = team.robots.iter().find(|robot| robot.id == id) else {
        bail!(r#"ID "{id}" not found in team.toml"#);
    };
    progress_bar.set_prefix(format!("[{robot} {}]", team_robot.hostname));
    let ip = format!("10.1.{}.{}", team.team_number, team_robot.number);

    const CONNECTION_NAME: &str = "Wired Connection 2";
    const INTERFACE: &str = "eth2";
    robot.ssh_to_robot()?.arg(format!(
        r#"nmcli connection modify "{CONNECTION_NAME}" ipv4.addresses "{ip}/24, 192.168.10.102/24""#,
    )).ssh_with_log("setting static IP",& progress_bar).await?;
    robot
        .ssh_to_robot()?
        .arg(format!("nmcli device reapply {INTERFACE}"))
        .ssh_with_log("applying network configuration", &progress_bar)
        .await?;

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

    if let Some(image_file) = image_file {
        const REMOTE_IMAGE_PATH: &str = "/home/booster/.cache/hulk/runtime-container-image.tar";
        robot
            .rsync_with_robot()?
            .arg(image_file)
            .arg(format!("{}:{REMOTE_IMAGE_PATH}", robot.address))
            .rsync_with_log("uploading podman image", &progress_bar)
            .await?;
        robot
            .ssh_to_robot()?
            .arg(format!("sudo podman load -i {REMOTE_IMAGE_PATH}"))
            .ssh_with_log("loading podman image", &progress_bar)
            .await?;
    }

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
                    .wrap_err_with(|| format!("failed at {name}"))
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
