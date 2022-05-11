use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::{bail, Context};
use log::info;

use crate::naossh;

pub async fn shutdown(nao: Ipv4Addr, is_reboot: bool, project_root: PathBuf) -> anyhow::Result<()> {
    let command = if is_reboot {
        "systemctl reboot"
    } else {
        "systemctl poweroff"
    };
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Sending '{}' to {} failed", command, nao))?;
    if output.exit_status != Some(0) {
        bail!(
            "Shutdown on {} failed with exit status: {:?}",
            nao,
            output.exit_status
        )
    }
    if is_reboot {
        info!("Rebooted {}", nao);
    } else {
        info!("Shut {} down", nao);
    }
    Ok(())
}
