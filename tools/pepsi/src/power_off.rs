use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use futures_util::{stream::FuturesUnordered, StreamExt};
use nao::Nao;

use crate::{
    parsers::{number_to_ip, Connection, NaoAddress},
    progress_indicator::ProgressIndicator,
};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to power off e.g. 20w or 10.1.24.22
    #[arg(long)]
    pub naos: Option<Vec<NaoAddress>>,

    #[arg(long)]
    pub all: bool,
}

pub async fn power_off(arguments: Arguments) -> Result<()> {
    if arguments.all {
        let nao_number = 21..=37;

        nao_number
            .into_iter()
            .map(|nao_number| async move {
                if let Ok(nao) = try_from_number(nao_number).await {
                    nao.power_off().await.err();
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    } else {
        ProgressIndicator::map_tasks(
            arguments.naos.unwrap(),
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

pub async fn try_from_number(nao_number: u8) -> Result<Nao> {
    let host = number_to_ip(nao_number, Connection::Wired)?;
    if let Ok(nao) = Nao::try_new_with_ping(host).await {
        Ok(nao)
    } else {
        let host = number_to_ip(nao_number, Connection::Wireless)?;
        Nao::try_new_with_ping(host).await
    }
}
