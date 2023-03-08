use std::{
    fmt::{self, Display, Formatter},
    net::Ipv4Addr,
    path::Path,
};

use std::str::from_utf8;

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use constants::OS_VERSION;
use tokio::process::Command;

pub struct Nao {
    host: Ipv4Addr,
}

impl Nao {
    pub fn new(host: Ipv4Addr) -> Self {
        Self { host }
    }

    pub async fn has_stable_os_version(&self) -> bool {
        let output = self
            .ssh_to_nao()
            .arg("cat /etc/os-release")
            .output()
            .await
            .unwrap();
        let stdout = from_utf8(&output.stdout).unwrap();
        let Some(os_version) = extract_version_number(stdout) else {
            println!("No version on {} detected!", self.host);
            return false;
        };
        if os_version != OS_VERSION {
            println!("Unstable version on {}", self.host);
            println!("Use '--skip-has-stable-os-version' if you still want to proceed");
            println!("Installed OS: {os_version}, stable OS: {OS_VERSION}");
            return false;
        }

        true
    }

    fn get_ssh_flags(&self) -> Vec<String> {
        vec![
            "-lnao".to_string(),
            "-oLogLevel=quiet".to_string(),
            "-oStrictHostKeyChecking=no".to_string(),
            "-oUserKnownHostsFile=/dev/null".to_string(),
        ]
    }

    fn ssh_to_nao(&self) -> Command {
        let mut command = Command::new("ssh");
        for flag in self.get_ssh_flags() {
            command.arg(flag);
        }
        command.arg(self.host.to_string());
        command
    }

    fn rsync_with_nao(&self, mkpath: bool) -> Command {
        let mut command = Command::new("rsync");
        let ssh_flags = self.get_ssh_flags().join(" ");
        command
            .arg("--compress")
            .arg("--recursive")
            .arg("--times")
            .arg(format!("--rsh=ssh {ssh_flags}"));
        if mkpath {
            command.arg("--mkpath");
        }
        command
    }

    pub async fn execute_shell(&self) -> Result<()> {
        let status = self
            .ssh_to_nao()
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
            .ssh_to_nao()
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
            .ssh_to_nao()
            .arg("rm")
            .arg("--recursive")
            .arg("--force")
            .arg("/home/nao/hulk/logs/*")
            .status()
            .await
            .wrap_err("failed to remove the log directory")?;

        if !status.success() {
            bail!("rm ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn download_logs(&self, local_directory: impl AsRef<Path>) -> Result<()> {
        let status = self
            .ssh_to_nao()
            .arg("dmesg > /home/nao/hulk/logs/kernel.log")
            .status()
            .await
            .wrap_err("failed to write dmesg to kernel.log")?;

        if !status.success() {
            bail!("dmesg pipe ssh command exited with {status}");
        }

        let status = self
            .rsync_with_nao(true)
            .arg(format!("{}:hulk/logs/", self.host))
            .arg(local_directory.as_ref().to_str().unwrap())
            .status()
            .await
            .wrap_err("failed to execute rsync command")?;

        if !status.success() {
            bail!("rsync command exited with {status}");
        }

        Ok(())
    }

    pub async fn power_off(&self) -> Result<()> {
        let status = self
            .ssh_to_nao()
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
            .ssh_to_nao()
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
    ) -> Result<()> {
        let mut command = self.rsync_with_nao(true);
        command
            .arg("--keep-dirlinks")
            .arg("--copy-links")
            .arg(format!("{}/", local_directory.as_ref().display()))
            .arg(format!("{}:hulk/", self.host));

        if delete_remaining {
            command.arg("--delete").arg("--delete-excluded");
        }

        let status = command
            .status()
            .await
            .wrap_err("failed to execute rsync command")?;

        if !status.success() {
            bail!("rsync command exited with {status}");
        }

        Ok(())
    }

    pub async fn get_network_status(&self) -> Result<String> {
        let output = self
            .ssh_to_nao()
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
            .ssh_to_nao()
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

    pub async fn set_network(&self, network: Network) -> Result<()> {
        let command_string = [
            Network::SplA,
            Network::SplB,
            Network::SplC,
            Network::SplD,
            Network::SplE,
            Network::SplF,
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
        let mut command = self.ssh_to_nao();
        command.arg(command_string);
        let status = command
            .status()
            .await
            .wrap_err("failed to execute iwctl ssh command")?;

        if !status.success() {
            bail!("iwctl ssh command exited with {status}");
        }

        Ok(())
    }

    pub async fn flash_image(&self, image_path: impl AsRef<Path>) -> Result<()> {
        let status = self
            .rsync_with_nao(false)
            .arg(image_path.as_ref().to_str().unwrap())
            .arg(format!("{}:/data/.image/", self.host))
            .status()
            .await
            .wrap_err("failed to execute rsync command")?;

        if !status.success() {
            bail!("failed to upload image")
        }

        self.reboot().await
    }
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
        }
    }
}

fn extract_version_number(input: &str) -> Option<String> {
    let lines = input.lines();
    for line in lines {
        if line.contains("VERSION_ID") {
            let Some((_, os_version)) = line.split_once('=') else { continue; };
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
