use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::Context;
use log::info;
use tokio::{fs::create_dir_all, process::Command};

use crate::naossh;

pub async fn delete_logs(nao: Ipv4Addr, project_root: PathBuf) -> anyhow::Result<()> {
    info!("Deleting logs on {}", nao);
    let command = "rm -vfr /home/nao/naoqi/hulk* \
                           /home/nao/naoqi/filetransport_* \
                           /home/nao/naoqi/replay_* \
                           /media/usb/logs/*";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Deleting logs on {} failed", nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Deleting logs on {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Successfully deleted logs on {}", nao);
    Ok(())
}

pub async fn download_logs(
    nao: Ipv4Addr,
    log_download_directory: PathBuf,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    let log_directory = log_download_directory.join(nao.to_string());
    info!("Creating {:?} to store logs", log_directory);
    create_dir_all(&log_directory).await.with_context(|| {
        format!(
            "Failed to create log download target directory '{:?}'",
            log_directory
        )
    })?;
    info!("Downloading logs from {}", nao);
    let mut command = Command::new("rsync");
    command.args([
        "-trzP",
        "--include=replay_**",
        "--include=hulk*",
        "--include=filetransport_**",
        "--include=core.*",
        "--exclude=*",
        &format!(
            "--rsh=ssh -q -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -l nao -i {}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ),
        &format!("{}:naoqi/", nao),
        log_directory.to_str().unwrap(),
    ]);
    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to run rsync to download logs from {}", nao))?;
    if !output.status.success() {
        anyhow::bail!(
            "rsync to download logs from {} failed with exit status {}",
            nao,
            output.status
        )
    }
    Ok(())
}
