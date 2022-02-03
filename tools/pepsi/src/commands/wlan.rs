use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::Context;
use log::info;

use crate::naossh;

pub async fn get_wireless(nao: Ipv4Addr, project_root: PathBuf) -> anyhow::Result<String> {
    let command = "iwctl station wlan0 show";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Getting wireless information from {} failed", nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Getting wireless information from {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Successfully got wireless information from {}", nao);
    Ok(output.stdout)
}

pub async fn get_networks(nao: Ipv4Addr, project_root: PathBuf) -> anyhow::Result<String> {
    let command = "iwctl station wlan0 get-networks";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Getting available networks from {} failed", nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Getting available networks from {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Successfully got available networks from {}", nao);
    Ok(output.stdout)
}

pub async fn connect_wireless(
    nao: Ipv4Addr,
    ssid: String,
    passphrase: Option<String>,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    info!("Connecting wireless on {} to {}", nao, ssid);
    let passphrase_parameter = match passphrase {
        Some(passphrase) => "--passphrase ".to_string() + &passphrase,
        None => "".to_string(),
    };
    let command = format!(
        "iwctl {} station wlan0 connect {}",
        passphrase_parameter, ssid
    );
    let output = naossh::command(nao, &command, &project_root)
        .await
        .with_context(|| format!("Connecting wireless on {} to {} failed", nao, ssid))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Connecting wireless on {} to {} failed with exit status {:?}",
            nao,
            ssid,
            output.exit_status
        )
    }
    info!("Successfully connected wireless on {} to {}", nao, ssid);
    Ok(())
}

pub async fn disconnect_wireless(nao: Ipv4Addr, project_root: PathBuf) -> anyhow::Result<()> {
    info!("Disconnecting wireless on {}", nao);
    let command = "iwctl station wlan0 disconnect";
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("Disconnecting wireless on {} failed", nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "Disconnecting wireless on {} failed with exit status {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Successfully disconnected wireless on {}", nao);
    Ok(())
}
