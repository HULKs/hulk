use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::Context;
use log::info;

use crate::naossh;

pub async fn shutdown(nao: Ipv4Addr, is_reboot: bool, project_root: PathBuf) -> anyhow::Result<()> {
    let command = if is_reboot {
        info!("Rebooting {}", nao);
        "systemctl reboot"
    } else {
        info!("Shutting down {}", nao);
        "systemctl poweroff"
    };
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Sending '{}' to {} failed", command, nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Shutdown on {} failed with exit status: {:?}",
            nao,
            output.exit_status
        )
    }
    info!(
        "Successful {} on {}",
        if is_reboot { "reboot" } else { "shutdown" },
        nao
    );
    Ok(())
}
