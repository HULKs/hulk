use std::{
    fmt::{Display, Formatter, Result as FormatResult},
    path::PathBuf,
};

use argument_parsers::NaoAddress;
use clap::{Args, ValueEnum};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};

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
    /// The NAOs to apply the postgame to, queried from the deploy.toml if not specified
    pub naos: Option<Vec<NaoAddress>>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Phase {
    GoldenGoal,
    FirstHalf,
    SecondHalf,
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        match self {
            Phase::GoldenGoal => write!(f, "golden-goal"),
            Phase::FirstHalf => write!(f, "first-half"),
            Phase::SecondHalf => write!(f, "second-half"),
        }
    }
}

pub async fn post_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;

    let all_naos = config.all_naos();
    let naos = if let Some(naos) = &arguments.naos {
        for nao in naos {
            if !all_naos.contains(nao) {
                bail!("NAO {nao} is not specified in the deploy.toml");
            }
        }
        naos
    } else {
        &all_naos
    };

    let log_directory = &arguments.log_directory.unwrap_or_else(|| {
        let log_directory_name = config.log_directory_name();

        repository.root.join("logs").join(log_directory_name)
    });

    ProgressIndicator::map_tasks(
        naos,
        "Executing postgame tasks...",
        |nao_address, progress_bar| async move {
            progress_bar.set_message("Pinging NAO...");
            let nao = Nao::ping_until_available(nao_address.ip).await;

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
            let log_directory = log_directory
                .join(arguments.phase.to_string())
                .join(nao_address.to_string());
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
