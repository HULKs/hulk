use std::{
    env::temp_dir,
    fmt::{self, Display, Formatter},
    fs::{set_permissions, Permissions},
    net::Ipv4Addr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use color_eyre::{
    eyre::{self, bail, eyre, WrapErr},
    Result,
};
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    select,
};

const PING_TIMEOUT: Duration = Duration::from_secs(2);

const BOOSTER_SSH_FLAGS: &[&str] = &[
    "-lbooster",
    "-oLogLevel=quiet",
    "-oStrictHostKeyChecking=no",
    "-oUserKnownHostsFile=/dev/null",
];

#[derive(Debug, Deserialize, Hash, Eq, PartialEq)]
#[serde(try_from = "String")]
pub struct RobotNumber {
    pub id: u8,
}

impl TryFrom<String> for RobotNumber {
    type Error = eyre::Error;

    fn try_from(value: String) -> Result<Self> {
        let id = value
            .parse()
            .wrap_err_with(|| format!("failed to parse `{value}` into Robot number"))?;
        Ok(Self { id })
    }
}

pub struct Robot {
    pub address: Ipv4Addr,
}

impl Robot {
    pub fn new(address: Ipv4Addr) -> Self {
        Self { address }
    }

    pub async fn ping_until_available(host: Ipv4Addr) -> Self {
        loop {
            if let Ok(robot) = Self::try_new_with_ping(host).await {
                return robot;
            }
        }
    }

    pub async fn try_new_with_ping(host: Ipv4Addr) -> Result<Self> {
        Self::try_new_with_ping_and_arguments(host, PING_TIMEOUT).await
    }

    pub async fn try_new_with_ping_and_arguments(
        host: Ipv4Addr,
        timeout: Duration,
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        const TIMEOUT_FLAG: &str = "-t";
        #[cfg(not(target_os = "macos"))]
        const TIMEOUT_FLAG: &str = "-w";

        match Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg(TIMEOUT_FLAG)
            .arg(timeout.as_secs().to_string())
            .arg(host.to_string())
            .output()
            .await
        {
            Ok(output) if output.status.success() => Ok(Self::new(host)),
            _ => bail!("No route to {host}"),
        }
    }

    pub async fn get_os_version(&self) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("cat /opt/booster/version.txt")
            .output()
            .await
            .wrap_err("failed to execute cat ssh command")?;

        let stdout = String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")?;
        extract_version_number(&stdout).ok_or_else(|| eyre!("could not extract version number"))
    }

    fn create_login_script() -> Result<PathBuf> {
        let path = temp_dir().join("booster_login_script");

        std::fs::write(&path, b"#!/usr/bin/env sh\necho 123456")
            .wrap_err("failed to write to robot login script")?;

        #[cfg(unix)]
        {
            set_permissions(&path, Permissions::from_mode(0o755))
                .wrap_err("failed to set permissions")?;
        }

        Ok(path)
    }

    pub fn ssh_to_robot(&self) -> Result<Command> {
        let temp_file = Self::create_login_script().wrap_err("failed to create login script")?;

        let mut command = Command::new("ssh");

        command.env("SSH_ASKPASS", temp_file.as_os_str());
        command.env("SSH_ASKPASS_REQUIRE", "force");

        for flag in BOOSTER_SSH_FLAGS {
            command.arg(flag);
        }
        command.arg(self.address.to_string());
        Ok(command)
    }

    pub fn rsync_with_robot(&self) -> Result<Command> {
        let mut command = Command::new("rsync");

        let temp_file = Self::create_login_script().wrap_err("failed to create login script")?;

        command.env("SSH_ASKPASS", temp_file.as_os_str());
        command.env("SSH_ASKPASS_REQUIRE", "force");

        let ssh_flags = BOOSTER_SSH_FLAGS.join(" ");
        command
            .stdout(Stdio::piped())
            .arg("--recursive")
            .arg("--times")
            .arg("--no-inc-recursive")
            .arg("--human-readable")
            .arg(format!("--rsh=ssh {ssh_flags}"));
        Ok(command)
    }

    pub async fn execute_shell(&self) -> Result<()> {
        let status = self
            .ssh_to_robot()?
            .status()
            .await
            .wrap_err("failed to execute shell ssh command")?;

        if !status.success() {
            bail!("shell ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn execute_systemctl(&self, action: SystemctlAction, unit: &str) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("systemctl")
            .arg("--user")
            .arg(match action {
                SystemctlAction::Disable => "disable",
                SystemctlAction::Enable => "enable",
                SystemctlAction::Restart => "restart",
                SystemctlAction::Start => "start",
                SystemctlAction::Status => "status",
                SystemctlAction::Stop => "stop",
            })
            .arg(unit)
            .output()
            .await
            .wrap_err("failed to execute systemctl ssh command")?;

        let status = output.status;

        if !status.success() {
            let systemctl_status_successful = matches!(action, SystemctlAction::Status)
                && status
                    .code()
                    .ok_or_else(|| eyre!("failed to extract exit code from {status:?}"))?
                    != 255;
            if !systemctl_status_successful {
                bail!("systemctl ssh command exited with {status}");
            }
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn delete_logs(&self) -> Result<()> {
        let status = self
            .ssh_to_robot()?
            .arg("rm")
            .arg("-r")
            .arg("-f")
            .arg("/home/robot/hulk/logs/*")
            .status()
            .await
            .wrap_err("failed to remove the log directory")?;

        if !status.success() {
            bail!("rm ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn download_logs(
        &self,
        local_directory: impl AsRef<Path>,
        progress_callback: impl Fn(&str),
    ) -> Result<()> {
        let status = self
            .ssh_to_robot()?
            .arg("sudo dmesg > /home/robot/hulk/logs/kernel.log")
            .status()
            .await
            .wrap_err("failed to write dmesg to kernel.log")?;

        if !status.success() {
            bail!("dmesg pipe ssh command exited with {status}");
        }

        let rsync = self
            .rsync_with_robot()?
            .arg("--mkpath")
            .arg("--info=progress2")
            .arg(format!("{}:hulk/logs/", self.address))
            .arg(local_directory.as_ref().to_str().unwrap())
            .spawn()
            .wrap_err("failed to execute rsync command")?;

        monitor_rsync_progress_with(rsync, progress_callback).await
    }

    pub async fn list_logs(&self) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("ls")
            .arg("hulk/logs/*")
            .output()
            .await
            .wrap_err("failed to execute list command")?;

        if !output.status.success() {
            bail!("list ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn retrieve_logs(&self) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("tail")
            .arg("-n+1")
            .arg("hulk/logs/hulk.{out,err}")
            .output()
            .await
            .wrap_err("failed to execute cat command")?;

        if !output.status.success() {
            bail!("cat ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn power_off(&self) -> Result<()> {
        let status = self
            .ssh_to_robot()?
            .arg("systemctl")
            .arg("poweroff")
            .status()
            .await
            .wrap_err("failed to execute poweroff ssh command")?;

        if !status.success() {
            bail!("poweroff ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn reboot(&self) -> Result<()> {
        let status = self
            .ssh_to_robot()?
            .arg("systemctl")
            .arg("reboot")
            .status()
            .await
            .wrap_err("failed to execute reboot ssh command")?;

        if !status.success() {
            bail!("reboot ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn upload(
        &self,
        local_directory: impl AsRef<Path>,
        remote_directory: impl AsRef<Path>,
        delete_remaining: bool,
        progress_callback: impl Fn(&str),
    ) -> Result<()> {
        let mut command = self.rsync_with_robot()?;
        command
            .arg("--mkpath")
            .arg("--copy-dirlinks")
            .arg("--copy-links")
            .arg("--info=progress2")
            .arg("--exclude=.git")
            .arg("--exclude=webots")
            .arg("--exclude=tools/machine-learning")
            .arg("--exclude=tools/charging-box")
            .arg("--filter=dir-merge,- .gitignore")
            .arg(format!("{}/", local_directory.as_ref().display()))
            .arg(format!(
                "{}:{}/",
                self.address,
                remote_directory.as_ref().display()
            ));

        if delete_remaining {
            command.arg("--delete").arg("--delete-excluded");
        }

        let rsync = command
            .spawn()
            .wrap_err("failed to execute rsync command")?;

        monitor_rsync_progress_with(rsync, progress_callback).await
    }

    pub async fn get_network_status(&self) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("nmcli")
            .arg("device")
            .arg("wifi")
            .arg("show")
            .output()
            .await
            .wrap_err("failed to execute nmcli ssh command")?;

        if !output.status.success() {
            bail!("nmcli ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn get_available_networks(&self) -> Result<String> {
        let output = self
            .ssh_to_robot()?
            .arg("nmcli")
            .arg("--colors yes")
            .arg("device")
            .arg("wifi")
            .arg("list")
            .output()
            .await
            .wrap_err("failed to execute nmcli ssh command")?;

        if !output.status.success() {
            bail!("nmcli ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn scan_networks(&self) -> Result<()> {
        let output = self
            .ssh_to_robot()?
            .arg("sudo")
            .arg("nmcli")
            .arg("device")
            .arg("wifi")
            .arg("rescan")
            .output()
            .await
            .wrap_err("failed to execute nmcli ssh command")?;

        if !output.status.success() {
            bail!("nmcli ssh command exited with {}", output.status);
        }

        Ok(())
    }

    pub async fn set_wifi(&self, network: Network) -> Result<()> {
        let command_string = Network::all()
            .into_iter()
            .map(|ssid| {
                format!(
                    "sudo nmcli connection modify {ssid} autoconnect {}",
                    if network == ssid { "yes" } else { "no" }
                )
            })
            .collect::<Vec<_>>()
            .join(" && ");
        let command_string = format!(
            "{command_string} && sudo nmcli {}",
            match network {
                Network::None => "device disconnect wlP1p1s0".to_string(),
                _ => format!("connection up {network}"),
            }
        );
        let status = self
            .ssh_to_robot()?
            .arg(command_string)
            .status()
            .await
            .wrap_err("failed to execute nmcli ssh command")?;

        if !status.success() {
            bail!("nmcli ssh command exited with {status}");
        }

        Ok(())
    }
}

impl Display for Robot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.address, formatter)
    }
}

async fn monitor_rsync_progress_with(
    mut process: Child,
    progress_callback: impl Fn(&str),
) -> Result<()> {
    let stdout = process
        .stdout
        .take()
        .expect("rsync did not have a handle to stdout");

    // rsync keeps printing on the same line, so we can't use `.lines()` here
    let mut stdout_reader = BufReader::new(stdout).split(b'\r');

    loop {
        select! {
            result = stdout_reader.next_segment() => {
                match result {
                    Ok(Some(line)) => {
                        match std::str::from_utf8(&line) {
                            Ok(line) => progress_callback(line),
                            Err(_error) => {},
                        }
                    },
                    _ => break,
                }
            }
            result = process.wait() => {
                if let Ok(status) = result {
                    if !status.success() {
                        bail!("failed to upload image")
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemctlAction {
    Disable,
    Enable,
    Restart,
    Start,
    Status,
    Stop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Network {
    None,
    HslA,
    HslB,
    HslC,
    HslD,
    HslE,
    HslF,
    HslHulks,
}

impl Display for Network {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Network::None => formatter.write_str("None"),
            Network::HslA => formatter.write_str("HSL_A"),
            Network::HslB => formatter.write_str("HSL_B"),
            Network::HslC => formatter.write_str("HSL_C"),
            Network::HslD => formatter.write_str("HSL_D"),
            Network::HslE => formatter.write_str("HSL_E"),
            Network::HslF => formatter.write_str("HSL_F"),
            Network::HslHulks => formatter.write_str("HSL_HULKs"),
        }
    }
}

impl Network {
    pub fn all() -> [Network; 7] {
        [
            Network::HslA,
            Network::HslB,
            Network::HslC,
            Network::HslD,
            Network::HslE,
            Network::HslF,
            Network::HslHulks,
        ]
    }
}

fn extract_version_number(input: &str) -> Option<String> {
    let lines = input.lines();
    let mut last_installed_version = None;
    for line in lines {
        if line.contains("Version: ") {
            let Some((_, os_version)) = line.split_once(": ") else {
                continue;
            };
            last_installed_version = Some(os_version.to_string());
        }
    }

    last_installed_version
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn matches_stable_os_version() {
        let input = r#"------------------
Version: v1.3.1.3-release
Branch: release/v1.3.0.3-0918
Commit ID: 648875f34d6dbc7cf3c25756b726bafe0366612b
Install time: Wed Dec  3 11:40:46 AM CST 2025
------------------
Version: v1.5.0.9-release-0387-2026-01-23
Branch: branch-2026-01-23T22-01-38.478509+0800-IU0
Commit ID: 4c563561c0ad5288b8994b4206a2a7dc9d42c1da
Install time: Thu Feb 12 01:51:44 AM CST 2026"#;

        let output = extract_version_number(input);
        assert_eq!(output, Some("v1.5.0.9-release-0387-2026-01-23".to_string()));
    }
}
