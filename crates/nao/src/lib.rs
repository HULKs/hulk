use std::{
    env::temp_dir,
    fmt::{self, Display, Formatter},
    fs::{set_permissions, Permissions},
    net::Ipv4Addr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
};

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    select,
};

pub const PING_TIMEOUT_SECONDS: u32 = 2;

pub struct Nao {
    pub host: Ipv4Addr,
}

impl Nao {
    pub fn new(host: Ipv4Addr) -> Self {
        Self { host }
    }

    pub async fn try_new_with_ping(host: Ipv4Addr) -> Result<Self> {
        Self::try_new_with_ping_and_arguments(host, PING_TIMEOUT_SECONDS).await
    }

    pub async fn try_new_with_ping_and_arguments(
        host: Ipv4Addr,
        timeout_seconds: u32,
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        const TIMEOUT_FLAG: &str = "-t";
        #[cfg(not(target_os = "macos"))]
        const TIMEOUT_FLAG: &str = "-w";

        match Command::new("ping")
            .arg("-c")
            .arg("1")
            .arg(TIMEOUT_FLAG)
            .arg(timeout_seconds.to_string())
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
            .ssh_to_nao()?
            .arg("cat /etc/os-release")
            .output()
            .await
            .wrap_err("failed to execute cat ssh command")?;

        let stdout = String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")?;
        extract_version_number(&stdout).ok_or_else(|| eyre!("could not extract version number"))
    }

    fn get_ssh_flags(&self) -> Vec<String> {
        vec![
            "-lnao".to_string(),
            "-oLogLevel=quiet".to_string(),
            "-oStrictHostKeyChecking=no".to_string(),
            "-oUserKnownHostsFile=/dev/null".to_string(),
        ]
    }

    fn create_login_script() -> Result<PathBuf> {
        let path = temp_dir().join("nao_login_script");

        std::fs::write(&path, b"#!/usr/bin/env sh\necho nao")
            .wrap_err("failed to write to nao login script")?;

        #[cfg(unix)]
        {
            set_permissions(&path, Permissions::from_mode(0o755))
                .wrap_err("failed to set permissions")?;
        }

        Ok(path)
    }

    fn ssh_to_nao(&self) -> Result<Command> {
        let temp_file =
            Self::create_login_script().wrap_err("failed to create ssh login script")?;

        let mut command = Command::new("ssh");
        command.env("SSH_ASKPASS", temp_file.as_os_str());
        command.env("SSH_ASKPASS_REQUIRE", "force");

        for flag in self.get_ssh_flags() {
            command.arg(flag);
        }
        command.arg(self.host.to_string());

        Ok(command)
    }

    pub fn rsync_with_nao(&self, mkpath: bool) -> Command {
        let mut command = Command::new("rsync");
        let ssh_flags = self.get_ssh_flags().join(" ");
        command
            .stdout(Stdio::piped())
            .arg("--recursive")
            .arg("--times")
            .arg("--no-inc-recursive")
            .arg("--human-readable")
            .arg(format!("--rsh=ssh {ssh_flags}"));
        if mkpath {
            command.arg("--mkpath");
        }
        command
    }

    pub async fn execute_shell(&self) -> Result<()> {
        let status = self
            .ssh_to_nao()?
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
            .ssh_to_nao()?
            .arg("systemctl")
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
            .ssh_to_nao()?
            .arg("rm")
            .arg("-r")
            .arg("-f")
            .arg("/home/nao/hulk/logs/*")
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
            .ssh_to_nao()?
            .arg("sudo dmesg > /home/nao/hulk/logs/kernel.log")
            .status()
            .await
            .wrap_err("failed to write dmesg to kernel.log")?;

        if !status.success() {
            bail!("dmesg pipe ssh command exited with {status}");
        }

        let rsync = self
            .rsync_with_nao(true)
            .arg("--info=progress2")
            .arg(format!("{}:hulk/logs/", self.host))
            .arg(local_directory.as_ref().to_str().unwrap())
            .spawn()
            .wrap_err("failed to execute rsync command")?;

        monitor_rsync_progress_with(rsync, progress_callback).await
    }

    pub async fn list_logs(&self) -> Result<String> {
        let output = self
            .ssh_to_nao()?
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
            .ssh_to_nao()?
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
            .ssh_to_nao()?
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
            .ssh_to_nao()?
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
        delete_remaining: bool,
        progress_callback: impl Fn(&str),
    ) -> Result<()> {
        let mut command = self.rsync_with_nao(true);
        command
            .arg("--keep-dirlinks")
            .arg("--copy-links")
            .arg("--info=progress2")
            .arg(format!("{}/", local_directory.as_ref().display()))
            .arg(format!("{}:hulk/", self.host));

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
            .ssh_to_nao()?
            .arg("iwctl")
            .arg("station")
            .arg("wlan0")
            .arg("show")
            .output()
            .await
            .wrap_err("failed to execute iwctl ssh command")?;

        if !output.status.success() {
            bail!("iwctl ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn get_available_networks(&self) -> Result<String> {
        let output = self
            .ssh_to_nao()?
            .arg("iwctl")
            .arg("station")
            .arg("wlan0")
            .arg("get-networks")
            .output()
            .await
            .wrap_err("failed to execute iwctl ssh command")?;

        if !output.status.success() {
            bail!("iwctl ssh command exited with {}", output.status);
        }

        String::from_utf8(output.stdout).wrap_err("failed to decode UTF-8")
    }

    pub async fn scan_networks(&self) -> Result<()> {
        let output = self
            .ssh_to_nao()?
            .arg("iwctl")
            .arg("station")
            .arg("wlan0")
            .arg("scan")
            .output()
            .await
            .wrap_err("failed to execute iwctl ssh command")?;

        if !output.status.success() {
            bail!("iwctl ssh command exited with {}", output.status);
        }

        Ok(())
    }

    pub async fn set_network(&self, network: Network) -> Result<()> {
        let command_string = [
            Network::SplA,
            Network::SplB,
            Network::SplC,
            Network::SplD,
            Network::SplE,
            Network::SplF,
            Network::SplHulks,
        ]
        .into_iter()
        .map(|possible_network| {
            format!(
                "iwctl known-networks {possible_network} set-property AutoConnect {}",
                if network == possible_network {
                    "yes"
                } else {
                    "no"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(" && ");
        let command_string = format!(
            "{command_string} && iwctl station wlan0 {}",
            match network {
                Network::None => "disconnect".to_string(),
                _ => format!("connect {network}"),
            }
        );
        let status = self
            .ssh_to_nao()?
            .arg(command_string)
            .status()
            .await
            .wrap_err("failed to execute iwctl ssh command")?;

        if !status.success() {
            bail!("iwctl ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn flash_image(
        &self,
        image_path: impl AsRef<Path>,
        progress_callback: impl Fn(&str),
    ) -> Result<()> {
        let rsync = self
            .rsync_with_nao(false)
            .arg("--copy-links")
            .arg("--info=progress2")
            .arg(image_path.as_ref().to_str().unwrap())
            .arg(format!("{}:/data/.image/", self.host))
            .spawn()
            .wrap_err("failed to execute rsync command")?;

        monitor_rsync_progress_with(rsync, progress_callback).await?;
        Ok(())
    }
}

impl Display for Nao {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.host, formatter)
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
    SplA,
    SplB,
    SplC,
    SplD,
    SplE,
    SplF,
    SplHulks,
}

impl Display for Network {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Network::None => formatter.write_str("None"),
            Network::SplA => formatter.write_str("SPL_A"),
            Network::SplB => formatter.write_str("SPL_B"),
            Network::SplC => formatter.write_str("SPL_C"),
            Network::SplD => formatter.write_str("SPL_D"),
            Network::SplE => formatter.write_str("SPL_E"),
            Network::SplF => formatter.write_str("SPL_F"),
            Network::SplHulks => formatter.write_str("SPL_HULKs"),
        }
    }
}

fn extract_version_number(input: &str) -> Option<String> {
    let lines = input.lines();
    for line in lines {
        if line.contains("VERSION_ID") {
            let Some((_, os_version)) = line.split_once('=') else {
                continue;
            };
            return Some(os_version.to_string());
        }
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn matches_stable_os_version() {
        let input = r#"ID=hulks-os
    NAME="HULKs-OS"
    VERSION="5.1.3 (langdale)"
    VERSION_ID=5.1.3
    PRETTY_NAME="HULKs-OS 5.1.3 (langdale)"
    DISTRO_CODENAME="langdale"#;

        let output = extract_version_number(input);
        assert_eq!(output, Some("5.1.3".to_string()));
    }
}
