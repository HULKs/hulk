use std::{
    fmt::Write,
    path::{Path, PathBuf},
    process::Stdio,
    str,
};

use clap::Args;
use color_eyre::{
    Result,
    eyre::{WrapErr, bail, eyre},
};

use argument_parsers::RobotAddress;
use indicatif::ProgressBar;
use repository::{Repository, team::Team};
use robot::{Network, Robot};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    select,
};

use crate::progress_indicator::ProgressIndicator;

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

static PACKAGES: [&str; 2] = ["zenohd", "zenoh-bridge-dds"];

const WIFI_PASSWORD: &str = "HSL?!HSL?!";

const PODMAN_INSTALLATION_SCRIPT: &str = include_str!("install-podman.sh");

static ADD_ZENOH_APT_SOURCES: &str = "
curl -L https://download.eclipse.org/zenoh/debian-repo/zenoh-public-key | sudo gpg --dearmor --yes --output /etc/apt/keyrings/zenoh-public-key.gpg
grep \"https://download.eclipse.org/zenoh/debian-repo/\" /etc/apt/sources.list || echo \"deb [signed-by=/etc/apt/keyrings/zenoh-public-key.gpg] https://download.eclipse.org/zenoh/debian-repo/ /\" | sudo tee -a /etc/apt/sources.list > /dev/null
";

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    let setup_path = &repository.root.join("tools/k1-setup");

    let progress = ProgressIndicator::new();

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
                    &team,
                    setup_path,
                )
            },
        )
        .await;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn gammaray_robot(
    robot: RobotAddress,
    progress_bar: ProgressBar,
    password: &str,
    image_file: Option<&Path>,
    team: &Team,
    setup: &Path,
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
        .arg(format!(
            "echo {} | sudo tee /etc/hostname > /dev/null",
            team_robot.hostname
        ))
        .ssh_with_log("setting hostname", &progress_bar)
        .await?;

    set_up_static_ips(&robot, team_robot, team.team_number, &progress_bar).await?;
    set_up_wifi(&robot, team_robot, team.team_number, &progress_bar).await?;

    robot
        .ssh_to_robot()?
        .arg(ADD_ZENOH_APT_SOURCES)
        .ssh_with_log("adding zenoh APT sources", &progress_bar)
        .await?;

    let mut check_string = "(".to_string();
    for package in PACKAGES {
        check_string.push_str(&format!("dpkg -s {package} && "));
    }
    check_string.push_str("true)");
    robot
        .ssh_to_robot()?
        // check if packages are installed already
        .arg(check_string)
        .arg("|| (")
        .arg("sudo apt update && sudo apt install -y")
        .args(PACKAGES)
        .arg(")")
        .ssh_with_log("installing packages", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo nvidia-ctk cdi generate --output=/etc/cdi/nvidia.yaml")
        .ssh_with_log("generating nvidia CDI", &progress_bar)
        .await?;

    install_podman(&robot, &progress_bar)
        .await
        .wrap_err("failed to install podman on robot")?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("zenohd.json5"))
        .arg(format!("{}:/etc/zenohd/zenohd.json5", robot.address))
        .rsync_with_log("uploading zenohd config", &progress_bar)
        .await?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("zenoh-bridge-dds.json5"))
        .arg(format!(
            "{}:/etc/zenoh-bridge-dds/conf.json5",
            robot.address
        ))
        .rsync_with_log("uploading zenoh-bridge-dds config", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo mkdir -p /etc/systemd/system/zenoh-bridge-dds.service.d/")
        .ssh_with_log(
            "creating zenoh-bridge-dds systemd override directory",
            &progress_bar,
        )
        .await?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("zenoh-bridge-dds.service.override.conf"))
        .arg(format!(
            "{}:/etc/systemd/system/zenoh-bridge-dds.service.d/override.conf",
            robot.address
        ))
        .rsync_with_log("uploading zenoh-bridge-dds service override", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl daemon-reload")
        .ssh_with_log("reloading service daemon", &progress_bar)
        .await?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("hulk.service"))
        .arg(format!("{}:/etc/systemd/system/", robot.address))
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
            .arg("--info=progress2")
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

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("hulk"))
        .arg(setup.join("launch-hulk"))
        .arg(format!("{}:/usr/bin/", robot.address))
        .rsync_with_log("uploading binaries", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl daemon-reload")
        .ssh_with_log("reloading service daemon", &progress_bar)
        .await?;

    robot
        .rsync_with_robot()?
        .arg("--rsync-path=sudo rsync")
        .arg("--info=progress2")
        .arg(setup.join("hulk-runtime.container"))
        .arg(format!("{}:/etc/containers/systemd/", robot.address))
        .rsync_with_log("uploading service file", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl enable --now")
        .arg("zenohd")
        .ssh_with_log("enabling zenohd services", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl enable --now")
        .arg("zenoh-bridge-dds")
        .ssh_with_log("enabling zenoh-bridge-dds services", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl enable --now")
        .arg("hulk")
        .ssh_with_log("enabling hulk", &progress_bar)
        .await?;

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl disable --now")
        .args([
            "booster-daemon-perception",
            "booster-agent-manager",
            "booster-lui",
            "booster-rtc-speech",
        ])
        .ssh_with_log("disabling booster services", &progress_bar)
        .await?;

    Ok(())
}

async fn install_podman(robot: &Robot, progress_bar: &ProgressBar) -> Result<()> {
    robot
        .ssh_to_robot()?
        .arg(format!(
            "sudo bash -s <<'EOF'\n{}\nEOF",
            PODMAN_INSTALLATION_SCRIPT
        ))
        .ssh_with_log("installing podman", progress_bar)
        .await
        .wrap_err("podman install/verification failed")
}

async fn set_up_static_ips(
    robot: &Robot,
    team_robot: &repository::team::Robot,
    team_number: u8,
    progress_bar: &ProgressBar,
) -> Result<()> {
    let ethernet_ip = format!("10.1.{}.{}", team_number, team_robot.number);
    const CONNECTION_NAME: &str = "Wired connection 2";
    const INTERFACE: &str = "enP9p1s0";

    robot.ssh_to_robot()?.arg(format!(
        // Set the new IP but preserve the 192.168.10.102 which is necessary for services on the
        // robot to work
        r#"sudo nmcli connection modify "{CONNECTION_NAME}" ipv4.addresses "{ethernet_ip}/24, 192.168.10.102/24" ipv4.gateway 10.1.24.1"#,
    )).ssh_with_log("setting static IP", progress_bar).await?;

    robot
        .ssh_to_robot()?
        // Unlike up/down-ing the connection, reapply doesn't break existing connections
        .arg(format!("sudo nmcli device reapply {INTERFACE}"))
        .ssh_with_log("applying network configuration", progress_bar)
        .await
}

async fn set_up_wifi(
    robot: &Robot,
    team_robot: &repository::team::Robot,
    team_number: u8,
    progress_bar: &ProgressBar,
) -> Result<()> {
    for network in Network::all() {
        let ssid = network.to_string();
        let mut command = robot.ssh_to_robot()?;
        command.arg(format!(
            "sudo tee /etc/NetworkManager/system-connections/{ssid}.nmconnection > /dev/null \
             && sudo chmod 0600 /etc/NetworkManager/system-connections/{ssid}.nmconnection"
        ));
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.stdin(Stdio::piped());
        let mut child = command.spawn()?;
        let second_octet = match network {
            Network::HslA => 107,
            Network::HslB => 108,
            Network::HslC => 109,
            _ => 0,
        };
        let content = generate_nmconnection_file(
            &ssid,
            WIFI_PASSWORD,
            second_octet,
            team_number,
            team_robot.number,
        );
        child
            .stdin
            .take()
            .expect("child had no stdin")
            .write_all(content.as_bytes())
            .await?;
        fail_on_non_zero_exit_code(child)
            .await
            .wrap_err_with(|| format!("failed to create NetworkManager config for {ssid}"))?;
    }

    robot
        .ssh_to_robot()?
        .arg("sudo systemctl restart NetworkManager")
        .ssh_with_log("restarting NetworkManager", progress_bar)
        .await?;

    Ok(())
}

pub trait CommandExt {
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
        let mut stdout_lines = BufReader::new(process.stdout.take().unwrap()).split(line_delimiter);
        let mut stderr_lines = BufReader::new(process.stderr.take().unwrap()).split(line_delimiter);
        let mut stderr = String::new();

        loop {
            select! {
                Ok(Some(buffer)) = stdout_lines.next_segment() => {
                    if let Ok(text) = str::from_utf8(&buffer) {
                        let message = if name.is_empty() {
                            text.to_string()
                        } else {
                            format!("{name}: {text}")
                        };
                        progress_bar.set_message(message);
                    }
                }
                Ok(Some(buffer)) = stderr_lines.next_segment() => {
                    if let Ok(text) = str::from_utf8(&buffer) {
                        writeln!(&mut stderr, "{text}")?;
                    }
                }
                else => break,
            }
        }

        match process.wait().await?.code() {
            Some(0) => Ok(()),
            Some(code) => bail!("{name}: process exited with error code {code}\n{stderr}"),
            None => bail!("process was killed"),
        }
    }
}

async fn fail_on_non_zero_exit_code(
    mut process: tokio::process::Child,
) -> std::result::Result<(), color_eyre::eyre::Error> {
    let maybe_code = process.wait().await?.code();
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

fn generate_nmconnection_file(
    ssid: &str,
    password: &str,
    second_octet: u8,
    team_number: u8,
    robot_number: u8,
) -> String {
    let uuid = uuid::Uuid::new_v4();
    format!(
        "[connection]
id={ssid}
uuid={uuid}
type=wifi
autoconnect=false
interface-name=wlP1p1s0

[wifi]
mode=infrastructure
ssid={ssid}

[wifi-security]
auth-alg=open
key-mgmt=wpa-psk
psk={password}

[ipv4]
address1=10.{second_octet}.{team_number}.{robot_number}/16
method=manual

[ipv6]
method=disabled

[proxy]
"
    )
}
