use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::{bail, Context};
use log::info;
use tokio::{fs::create_dir_all, process::Command};

use crate::naossh;

pub async fn delete_logs(nao: Ipv4Addr, project_root: PathBuf) -> anyhow::Result<()> {
    let command = "rm -rf /home/nao/hulk/logs/*";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Failed to delete logs on {}", nao))?;
    if output.exit_status != Some(0) {
        bail!(
            "Deleting logs on {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Logs deleted on {}", nao);
    Ok(())
}

pub async fn download_logs(
    nao: Ipv4Addr,
    log_download_directory: PathBuf,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    let log_directory = log_download_directory.join(nao.to_string());
    create_dir_all(&log_directory).await.with_context(|| {
        format!(
            "Failed to create log download target directory '{:?}'",
            log_directory
        )
    })?;
    let command = "dmesg > hulk/logs/kernel.log";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Failed pull kernel logs on {}", nao))?;
    if output.exit_status != Some(0) {
        bail!(
            "Creating kernel logs on {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    let mut command = Command::new("rsync");
    command.args([
        "--times",
        "--recursive",
        "--compress",
        &format!(
            "--rsh=ssh -q -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -l nao -i {}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ),
        &format!("{}:hulk/logs/", nao),
        log_directory.to_str().unwrap(),
    ]);
    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to run rsync to download logs from {}", nao))?;
    if !output.status.success() {
        bail!(
            "rsync to download logs from {} failed with exit status {}",
            nao,
            output.status
        )
    }
    info!(
        "Logs downloaded from {} to {}",
        nao,
        log_directory.display()
    );
    Ok(())
}
