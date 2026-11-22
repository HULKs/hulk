use std::{
    fmt::Write,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::Stdio,
    str,
    thread::available_parallelism,
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

use self::overlay::RobotOverlayExt;
use crate::progress_indicator::ProgressIndicator;

mod bundle;
pub(crate) mod overlay;

#[derive(Args)]
pub struct Arguments {
    /// The robots to gammaray, e.g. 20w or 10.1.24.22
    #[arg(required_unless_present = "prepare")]
    robots: Vec<RobotAddress>,

    /// Discard the saved overlay and provision from a clean lower root
    #[arg(long, visible_alias = "sauber")]
    clean: bool,

    /// Ensure the offline runtime bundle exists,
    /// but exit without gammaraying robots
    #[arg(long, visible_alias = "vorbereit")]
    prepare: bool,

    /// Parallel compilation jobs per package during bundle preparation
    /// (defaults to the available CPUs minus two, at least one)
    #[arg(short, long, default_value_t = default_build_jobs())]
    jobs: NonZeroUsize,

    /// Build through Nix over SSH (defaults to REMOTE_WORKSPACE or .REMOTE_WORKSPACE)
    #[arg(long, num_args = 0..=1, value_name = "SSH_HOST", conflicts_with = "bundle")]
    remote: Option<Option<String>>,

    /// The password for the `booster` user
    #[arg(short, long, default_value = "123456")]
    password: String,

    /// Install a prepared runtime bundle without building or downloading
    #[arg(long, conflicts_with = "prepare")]
    bundle: Option<PathBuf>,

    /// Optional `update_x5` path
    /// e.g. `../update_x5`
    #[arg(short, long)]
    update_x5_file: Option<PathBuf>,
}

const WIFI_PASSWORD: &str = "HSL?!HSL?!";

const BUNDLE_INSTALLATION_SCRIPT: &str = include_str!("install-hulks-bundle.sh");
const REMOTE_BUNDLE: &str = "/home/booster/.cache/hulk/hulks";

const PRESERVE_BOOSTER_CONFIG_SCRIPT: &str = include_str!("gammaray/preserve-booster-config.sh");
const MANUFACTURER_SERVICES_SCRIPT: &str = include_str!("gammaray/manufacturer-services.sh");
const LOWER_ROOT_SCRIPT: &str = include_str!("gammaray/overlay-lower-root.sh");
const RESTORE_BOOSTER_TEMPLATE_SCRIPT: &str = include_str!("gammaray/restore-booster-template.sh");

fn default_build_jobs() -> NonZeroUsize {
    let cores = available_parallelism().unwrap_or(NonZeroUsize::MIN);
    NonZeroUsize::new(cores.get().saturating_sub(2)).unwrap_or(NonZeroUsize::MIN)
}

pub(super) fn manufacturer_services_command(mode: &str) -> String {
    format!(
        "set -e\nsudo -n unshare --mount bash -s <<'HULKS_BOOSTER_TEMPLATE'\n{LOWER_ROOT_SCRIPT}\n{RESTORE_BOOSTER_TEMPLATE_SCRIPT}\nHULKS_BOOSTER_TEMPLATE\nsudo bash -s -- {mode} <<'HULKS_MANUFACTURER_SERVICES'\n{PRESERVE_BOOSTER_CONFIG_SCRIPT}\n{MANUFACTURER_SERVICES_SCRIPT}\nHULKS_MANUFACTURER_SERVICES"
    )
}

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    let bundle = match arguments.bundle {
        Some(bundle) => {
            bundle::validate(&bundle, &repository.read_runtime_version().await?).await?
        }
        None => {
            let remote = match arguments.remote {
                Some(host) => Some(resolve_remote(repository, host).await?),
                None => None,
            };
            bundle::prepare(repository, arguments.jobs, remote.as_deref()).await?
        }
    };
    if arguments.prepare {
        return Ok(());
    }
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
                    arguments.clean,
                    arguments.update_x5_file.as_deref(),
                    &team,
                    &bundle,
                )
            },
        )
        .await;

    Ok(())
}

async fn resolve_remote(repository: &Repository, host: Option<String>) -> Result<String> {
    let host = match host {
        Some(host) => host,
        None => {
            let workspace = match std::env::var("REMOTE_WORKSPACE") {
                Ok(value) if !value.is_empty() => value,
                Ok(_) | Err(std::env::VarError::NotPresent) => {
                    tokio::fs::read_to_string(repository.root.join(".REMOTE_WORKSPACE"))
                        .await
                        .wrap_err(
                            "--remote needs SSH_HOST, REMOTE_WORKSPACE, or .REMOTE_WORKSPACE",
                        )?
                }
                Err(error) => return Err(error).wrap_err("invalid REMOTE_WORKSPACE"),
            };
            let (host, path) = workspace.trim().split_once(':').ok_or_else(|| {
                eyre!("remote workspace must have the form user@host:path/to/repo")
            })?;
            if path.is_empty() {
                bail!("remote workspace path must not be empty");
            }
            host.to_owned()
        }
    };
    if !regex::Regex::new(r"^(?:[a-zA-Z0-9_][a-zA-Z0-9_.-]*@)?[a-zA-Z0-9_][a-zA-Z0-9_.-]*$")?
        .is_match(&host)
    {
        bail!("invalid remote SSH host {host:?}: expected [user@]hostname or an SSH alias");
    }
    Ok(host)
}

async fn gammaray_robot(
    robot: RobotAddress,
    progress_bar: ProgressBar,
    password: &str,
    clean: bool,
    update_x5_file: Option<&Path>,
    team: &Team,
    bundle: &Path,
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
    if !output.status.success() {
        bail!("failed to read robot ID");
    }
    let id = String::from_utf8(output.stdout)?;
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

    overlay::ensure(&robot, clean, &progress_bar).await?;

    robot
        .ssh_to_overlay()?
        .arg(format!(
            "echo {} | sudo tee /etc/hostname > /dev/null",
            team_robot.hostname
        ))
        .ssh_with_log("setting hostname", &progress_bar)
        .await?;

    deploy_nix(&robot, bundle, &progress_bar).await?;

    robot
        .ssh_to_overlay()?
        .arg("mkdir -p /home/booster/.cache/hulk/tensor-rt/ /home/booster/hulk/logs && sudo chown -R booster:booster /home/booster/.cache/hulk/tensor-rt")
        .ssh_with_log("preparing runtime directories", &progress_bar)
        .await?;

    if let Some(update_x5_file) = update_x5_file {
        robot
            .rsync_with_overlay()?
            .arg("--info=progress2")
            .arg(update_x5_file)
            .arg(format!("{}:.", robot.address))
            .rsync_with_log("uploading update_x5 binary", &progress_bar)
            .await?;
        robot
            .ssh_to_overlay()?
            .arg("sudo ./update_x5")
            .ssh_with_log("executing update_x5 binary", &progress_bar)
            .await?;
    }

    robot
        .ssh_to_overlay()?
        .arg("sudo systemctl daemon-reload")
        .ssh_with_log("reloading service daemon", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("sudo systemctl reload-or-restart hulks-firewall")
        .ssh_with_log("applying firewall rules", &progress_bar)
        .await?;

    // Stop replaced DDS publishers before starting the bridge.
    robot
        .ssh_to_overlay()?
        .arg(format!(
            "SSHPASS='br{}' /nix/var/nix/profiles/hulks/bin/sshpass -e ssh -o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o NumberOfPasswordPrompts=1 root@192.168.127.10 'systemctl disable --now cam_rectify_stream.service'",
            password.replace('\'', "'\\''")
        ))
        .ssh_with_log("disabling x5 camera rectify service", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg(manufacturer_services_command("disable"))
        .ssh_with_log("disabling Booster services", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("sudo systemctl restart zenohd")
        .ssh_with_log("restarting zenohd service", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("sudo systemctl restart zenoh-bridge-dds")
        .ssh_with_log("restarting zenoh-bridge-dds service", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("sudo systemctl start jetson-clocks-refresh.timer")
        .ssh_with_log("starting jetson clocks refresh timer", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("if /usr/bin/launch-hulk --check; then sudo systemctl restart hulk; fi")
        .ssh_with_log("starting hulk if uploaded", &progress_bar)
        .await?;

    robot
        .ssh_to_overlay()?
        .arg("for service in hulks-firewall zenohd zenoh-bridge-dds; do sudo systemctl is-active \"$service\" || exit 1; done; if /usr/bin/launch-hulk --check >/dev/null 2>&1; then sudo systemctl is-active hulk; fi")
        .ssh_with_log("verifying services", &progress_bar)
        .await?;

    set_up_wifi(&robot, team_robot, team.team_number, &progress_bar).await?;
    set_up_static_ips(&robot, team_robot, team.team_number, &progress_bar).await?;

    Ok(())
}

async fn deploy_nix(robot: &Robot, bundle: &Path, progress_bar: &ProgressBar) -> Result<()> {
    robot
        .ssh_to_overlay()?
        .arg(format!("mkdir -p {REMOTE_BUNDLE}"))
        .ssh_with_log("creating package cache", progress_bar)
        .await?;
    robot
        .rsync_with_overlay()?
        .arg("--delete")
        .arg("--info=progress2")
        .arg(bundle.join("."))
        .arg(format!("{}:{REMOTE_BUNDLE}/", robot.address))
        .rsync_with_log("uploading offline package bundle", progress_bar)
        .await?;
    robot
        .ssh_to_overlay()?
        .arg(format!(
            "sudo bash -s -- {REMOTE_BUNDLE} <<'HULKS_BUNDLE_INSTALL_SCRIPT'\n{BUNDLE_INSTALLATION_SCRIPT}\nHULKS_BUNDLE_INSTALL_SCRIPT"
        ))
        .ssh_with_log("installing nix bundle", progress_bar)
        .await
        .wrap_err("offline package installation failed")
}

async fn set_up_static_ips(
    robot: &Robot,
    team_robot: &repository::team::Robot,
    team_number: u8,
    progress_bar: &ProgressBar,
) -> Result<()> {
    let robot_number = team_robot.number;
    let ethernet_ip = format!("10.1.{}.{}", team_number, team_robot.number);
    const CONNECTION_NAME: &str = "Wired connection 2";

    robot.ssh_to_overlay()?.arg(format!(
        // Set the new IP but preserve the 192.168.10.102 which is necessary for services on the
        // robot to work
        r#"sudo nmcli connection modify "{CONNECTION_NAME}" ipv4.addresses "{ethernet_ip}/24, 192.168.10.102/24" ipv4.gateway 10.1.{team_number}.1 802-3-ethernet.cloned-mac-address "E6:4A:A0:37:19:{robot_number}""#,
    )).ssh_with_log("setting static IP", progress_bar).await?;

    robot
        .ssh_to_overlay()?
        // Unlike up/down-ing the connection, reapply doesn't break existing connections
        .arg("sudo nmcli connection up Wired\\ connection\\ 2")
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
        let mut command = robot.ssh_to_overlay()?;
        command.arg(format!(
            "sudo tee /etc/NetworkManager/system-connections/{ssid}.nmconnection > /dev/null \
             && sudo chmod 0600 /etc/NetworkManager/system-connections/{ssid}.nmconnection"
        ));
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.stdin(Stdio::piped());
        let mut child = command.spawn()?;
        const WIFI_SECOND_OCTET: u8 = 0;
        let content = generate_nmconnection_file(
            &ssid,
            WIFI_PASSWORD,
            WIFI_SECOND_OCTET,
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
        .ssh_to_overlay()?
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
        let mut stderr_lines = BufReader::new(process.stderr.take().unwrap()).split(b'\n');
        let mut stdout = String::new();
        let mut stderr = String::new();

        loop {
            select! {
                Ok(Some(buffer)) = stdout_lines.next_segment() => {
                    if let Ok(text) = str::from_utf8(&buffer) {
                        writeln!(&mut stdout, "{text}")?;
                        let message = if name.is_empty() {
                            text.to_string()
                        } else {
                            format!("{name}: {text}")
                        };
                        progress_bar.set_message(message);
                        if line_delimiter == b'\n' {
                            progress_bar.println(text);
                        }
                    }
                }
                Ok(Some(buffer)) = stderr_lines.next_segment() => {
                    if let Ok(text) = str::from_utf8(&buffer) {
                        writeln!(&mut stderr, "{text}")?;
                        progress_bar.println(text);
                    }
                }
                else => break,
            }
        }

        match process.wait().await?.code() {
            Some(0) => Ok(()),
            Some(code) => bail!(
                "{name}: process exited with error code {code}\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ),
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
# 2: disabled
powersave=2
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn offline_commands_need_no_signing_keys() -> color_eyre::Result<()> {
        crate::Arguments::try_parse_from(["pepsi", "gammaray", "--prepare"])?;
        crate::Arguments::try_parse_from([
            "pepsi",
            "gammaray",
            "42",
            "--bundle",
            "runtime-bundle",
        ])?;
        Ok(())
    }
}
