use std::path::PathBuf;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::NaoAddress;
use nao::{Nao, Network, SystemctlAction};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Do not disconnect from the WiFi network
    #[arg(long)]
    pub no_disconnect: bool,
    /// Directory where to store the downloaded logs (will be created if not existing)
    pub log_directory: PathBuf,
    /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn post_game(arguments: Arguments) -> Result<()> {
    let arguments = &arguments;
    ProgressIndicator::map_tasks(
        &arguments.naos,
        "Executing postgame tasks...",
        |nao_address, progress_bar| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            progress_bar.set_message("Stopping HULK service...");
            nao.execute_systemctl(SystemctlAction::Stop, "hulk")
                .await
                .wrap_err_with(|| format!("failed to execute systemctl hulk on {nao_address}"))?;

            progress_bar.set_message("Disconnecting from WiFi...");
            nao.set_wifi(Network::None)
                .await
                .wrap_err_with(|| format!("failed to set network on {nao_address}"))?;

            progress_bar.set_message("Downloading logs...");
            let log_directory = arguments.log_directory.join(nao_address.to_string());
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
