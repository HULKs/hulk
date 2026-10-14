use std::{
    fmt::{self, Display, Formatter},
    time::Duration,
};

use argument_parsers::RobotAddress;
use clap::{Args, ValueEnum};
use color_eyre::{Result, eyre::bail};
use indicatif::ProgressBar;
use robot::Robot;
use tokio::process::Command;
use tokio::time::{Instant, sleep, timeout};

use super::CommandExt;
use crate::progress_indicator::{InlineMessage, ProgressIndicator};

const STATUS: &str = include_str!("overlay-status.sh");
const PREPARE: &str = include_str!("overlay-prepare.sh");
const INIT: &str = include_str!("overlay-init.sh");
const LOWER_ROOT: &str = include_str!("overlay-lower-root.sh");
const BOOT_TARGET: &str = include_str!("hulks-overlay-boot.target");
const BOOT_SERVICE: &str = include_str!("hulks-overlay-boot.service");

#[derive(Args)]
pub(crate) struct Arguments {
    #[arg(value_enum)]
    action: Action,
    #[arg(required = true)]
    robots: Vec<RobotAddress>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Action {
    Enable,
    Disable,
    Status,
}

impl Display for Action {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Status => "status",
        })
    }
}

pub(crate) async fn overlay(arguments: Arguments) -> Result<()> {
    let action = arguments.action;
    ProgressIndicator::new()
        .map_tasks(arguments.robots, "", |address, progress| async move {
            let robot = Robot::try_new_with_ping(address.ip).await?;
            control(&robot, action, false, &progress)
                .await
                .map(InlineMessage)
        })
        .await;
    Ok(())
}

async fn control(
    robot: &Robot,
    action: Action,
    clean: bool,
    progress: &ProgressBar,
) -> Result<String> {
    let default = on_lower_root(robot, "systemctl --root=\"$root\" get-default").await?;
    let enabled = default.trim() == "hulks-overlay-boot.target";
    let message = match action {
        Action::Status => {
            let active = status(robot).await?.starts_with("overlay ");
            format!(
                "{}, {}",
                if active { "active" } else { "inactive" },
                if enabled { "enabled" } else { "disabled" }
            )
        }
        Action::Disable => {
            let mut commands = String::from("stage=${root%/}/var/lib/hulks-overlay\n");
            if enabled {
                commands.push_str(
                    "systemctl --root=\"$root\" set-default \"$(cat \"$stage/default-target\")\"\n",
                );
            }
            commands.push_str("rm -f \"$stage/clean\"\n");
            on_lower_root(robot, &commands).await?;
            "Overlay autostart: disabled".to_owned()
        }
        Action::Enable => {
            let mut install = format!(
                r#"stage=${{root%/}}/var/lib/hulks-overlay
units=${{root%/}}/etc/systemd/system
test "$(findmnt -n -o FSTYPE --target "$root")" = ext4
test ! -L "$stage"
mkdir -p "$stage"
chmod 700 "$stage"
for directory in upper work; do test ! -L "$stage/$directory"; done
cat > "$stage/init" <<'HULKS_OVERLAY_INIT'
{INIT}
HULKS_OVERLAY_INIT
cat > "$stage/prepare" <<'HULKS_OVERLAY_BOOT'
{PREPARE}
HULKS_OVERLAY_BOOT
chmod 755 "$stage/init" "$stage/prepare"
cat > "$units/hulks-overlay-boot.target" <<'HULKS_OVERLAY_TARGET'
{BOOT_TARGET}
HULKS_OVERLAY_TARGET
cat > "$units/hulks-overlay-boot.service" <<'HULKS_OVERLAY_SERVICE'
{BOOT_SERVICE}
HULKS_OVERLAY_SERVICE
"#
            );
            if !enabled {
                install.push_str(&format!(
                    "printf '%s\\n' '{}' > \"$stage/default-target\"\n",
                    default.trim().replace('\'', "'\\''")
                ));
            }
            if clean {
                install.push_str("touch \"$stage/clean\"\n");
            }
            install.push_str("systemctl --root=\"$root\" set-default hulks-overlay-boot.target\n");
            on_lower_root(robot, &install).await?;
            "Overlay autostart: enabled".to_owned()
        }
    };
    progress.set_message(message.clone());
    Ok(message)
}

async fn on_lower_root(robot: &Robot, commands: &str) -> Result<String> {
    let output = robot
        .ssh_to_robot()?
        .arg(format!(
            "sudo -n unshare --mount bash -s <<'HULKS_OVERLAY_CONTROL'\n{LOWER_ROOT}\n{commands}\nHULKS_OVERLAY_CONTROL"
        ))
        .output()
        .await?;
    if !output.status.success() {
        bail!(
            "lower-root command failed ({}):\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub(crate) async fn disable(robot: &Robot, progress: &ProgressBar) -> Result<()> {
    control(robot, Action::Disable, false, progress).await?;
    Ok(())
}

// Fail if root is not an overlay
const GUARD: &str = "test \"$(findmnt -n -o FSTYPE --mountpoint /)\" = overlay || exit 1;";

pub(super) trait RobotOverlayExt {
    fn ssh_to_overlay(&self) -> Result<Command>;
    fn rsync_with_overlay(&self) -> Result<Command>;
    fn rsync_with_overlay_sudo(&self) -> Result<Command>;
}

impl RobotOverlayExt for Robot {
    fn ssh_to_overlay(&self) -> Result<Command> {
        let mut command = self.ssh_to_robot()?;
        command.arg(GUARD);
        Ok(command)
    }

    fn rsync_with_overlay(&self) -> Result<Command> {
        let mut command = self.rsync_with_robot()?;
        command.arg(format!("--rsync-path={GUARD} rsync"));
        Ok(command)
    }

    fn rsync_with_overlay_sudo(&self) -> Result<Command> {
        let mut command = self.rsync_with_robot()?;
        command.arg(format!("--rsync-path={GUARD} sudo rsync"));
        Ok(command)
    }
}

async fn status(robot: &Robot) -> Result<String> {
    let output = timeout(
        Duration::from_secs(15),
        robot
            .ssh_to_robot()?
            .kill_on_drop(true)
            .arg(format!(
                "sudo -n sh -s <<'HULKS_OVERLAY_STATUS'\n{STATUS}\nHULKS_OVERLAY_STATUS"
            ))
            .output(),
    )
    .await??;
    if !output.status.success() {
        bail!(
            "overlay inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(super) async fn ensure(robot: &Robot, clean: bool, progress: &ProgressBar) -> Result<()> {
    progress.set_message("checking robot root filesystem");
    let initial = status(robot).await?;
    let Some((root, boot_id)) = initial.split_once(' ') else {
        bail!("unexpected overlay status: {initial}");
    };
    if !matches!(root, "normal" | "overlay") {
        bail!("unexpected overlay status: {initial}");
    }
    control(robot, Action::Enable, clean, progress).await?;
    if root == "overlay" && !clean {
        return Ok(());
    }
    robot
        .ssh_to_robot()?
        .arg("sudo -n systemd-run --unit=hulks-overlay-reboot --on-active=2s /bin/systemctl reboot")
        .ssh_with_log("rebooting into saved overlay", progress)
        .await?;

    progress.set_message("waiting for robot to restart into overlay");
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        sleep(Duration::from_secs(2)).await;
        if let Ok(current) = status(robot).await {
            if let Some(current_boot) = current.strip_prefix("overlay ")
                && current_boot != boot_id
            {
                return Ok(());
            }
        }
    }
    bail!("robot did not enter managed overlay within 180 seconds; provisioning was not started")
}
