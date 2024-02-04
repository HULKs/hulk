use std::str::FromStr;

use clap::Args;
use color_eyre::{
    eyre::{Report, WrapErr},
    Result,
};
use constants::HARDWARE_IDS;
use futures_util::{stream::FuturesUnordered, StreamExt};
use nao::Nao;

use crate::{
    parsers::{number_to_ip, Connection, NaoAddress},
    progress_indicator::ProgressIndicator,
};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to power off e.g. 20w, 10.1.24.22 or all
    #[arg(required = true)]
    pub naos: Vec<AddressOrAll>,
}

pub async fn power_off(arguments: Arguments) -> Result<()> {
    if arguments
        .naos
        .iter()
        .any(|address| matches!(address, AddressOrAll::All))
    {
        let addresses = HARDWARE_IDS
            .keys()
            .map(|&nao_number| async move { try_from_number(nao_number).await })
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
        let addresses = arguments.naos.iter().filter_map(|address| match address {
            AddressOrAll::Address(address) => Some(address),
            _ => None,
        });

        ProgressIndicator::map_tasks(
            addresses,
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

#[derive(Clone)]
pub enum AddressOrAll {
    Address(NaoAddress),
    All,
}

impl FromStr for AddressOrAll {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self> {
        if s == "all" {
            Ok(Self::All)
        } else {
            Ok(Self::Address(NaoAddress::from_str(s)?))
        }
    }
}
