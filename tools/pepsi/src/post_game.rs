use std::path::PathBuf;

use clap::{Args, ValueEnum};
use color_eyre::{eyre::WrapErr, Result};

use nao::{Nao, Network, SystemctlAction};
use repository::Repository;

use crate::{deploy_config::DeployConfig, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// Do not disconnect from the WiFi network
    #[arg(long)]
    pub no_disconnect: bool,
    /// Directory where to store the downloaded logs (will be created if not existing)
    #[arg(long)]
    pub log_directory: Option<PathBuf>,
    /// Current game phase
    #[arg(value_enum)]
    pub phase: Phase,
}

#[derive(Clone, ValueEnum)]
pub enum Phase {
    GoldenGoal,
    FirstHalf,
    SecondHalf,
}

pub async fn post_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;
    let naos = config.naos();

    let log_directory = &arguments.log_directory.unwrap_or_else(|| {
        let log_directory_name = config.log_directory_name();

        repository.root.join("logs").join(log_directory_name)
    });

    ProgressIndicator::map_tasks(
        &naos,
        "Executing postgame tasks...",
        |nao_address, progress_bar| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            progress_bar.set_message("Stopping HULK service...");
            nao.execute_systemctl(SystemctlAction::Stop, "hulk")
                .await
                .wrap_err_with(|| format!("failed to execute systemctl hulk on {nao_address}"))?;

            if !arguments.no_disconnect {
                progress_bar.set_message("Disconnecting from WiFi...");
                nao.set_wifi(Network::None)
                    .await
                    .wrap_err_with(|| format!("failed to set network on {nao_address}"))?;
            }

            progress_bar.set_message("Downloading logs...");
            let log_directory = log_directory.join(nao_address.to_string());
            nao.download_logs(log_directory, |status| {
                progress_bar.set_message(format!("Downloading logs: {status}"))
            })
            .await
            .wrap_err_with(|| format!("failed to download logs from {nao_address}"))?;

            Ok(())
        },
    )
    .await;

    Ok(())
}
