use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use constants::HARDWARE_IDS;
use futures_util::{stream::FuturesUnordered, StreamExt};
use nao::Nao;

use crate::{
    parsers::{number_to_ip, Connection, NaoAddress},
    progress_indicator::ProgressIndicator,
};

#[derive(Args)]
pub struct Arguments {
    /// Power off all NAOs
    #[arg(long)]
    pub all: bool,
    /// The NAOs to power off e.g. 20w or 10.1.24.22
    #[arg(required = true, conflicts_with = "all", num_args = 1..)]
    pub naos: Vec<NaoAddress>,
}

pub async fn power_off(arguments: Arguments) -> Result<()> {
    if arguments.all {
        let addresses = HARDWARE_IDS
            .keys()
            .map(|&nao_number| async move {
                let host = number_to_ip(nao_number, Connection::Wired)?;
                match Nao::try_new_with_ping(host).await {
                    Ok(nao) => Ok(nao),
                    Err(_) => {
                        let host = number_to_ip(nao_number, Connection::Wireless)?;
                        Nao::try_new_with_ping(host).await
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        ProgressIndicator::map_tasks(
            addresses.into_iter().filter_map(|nao| nao.ok()),
            "Powering off...",
            |nao, _progress_bar| async move {
                let nao = Nao::try_new_with_ping(nao.host).await?;
                nao.power_off()
                    .await
                    .wrap_err_with(|| format!("failed to power {nao} off"))
            },
        )
        .await;
    } else {
        ProgressIndicator::map_tasks(
            arguments.naos,
            "Powering off...",
            |nao_address, _progress_bar| async move {
                let nao = Nao::try_new_with_ping(nao_address.ip).await?;
                nao.power_off()
                    .await
                    .wrap_err_with(|| format!("failed to power {nao_address} off"))
            },
        )
        .await;
    }
    Ok(())
}
