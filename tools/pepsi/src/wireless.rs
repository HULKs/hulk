use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Subcommand,
};
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::{Nao, Network};

use crate::{
    parsers::{parse_network, NaoAddress, NETWORK_POSSIBLE_VALUES},
    results::gather_results,
};

#[derive(Subcommand)]
pub enum Arguments {
    /// List available networks
    List {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
    /// Set active network
    Set {
        /// The network to connect the wireless device to (None disconnects from anything)
        #[arg(
            value_parser = PossibleValuesParser::new(NETWORK_POSSIBLE_VALUES)
                .map(|s| parse_network(&s).unwrap()))
        ]
        network: Network,
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
    /// Show current network status
    Status {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub async fn wireless(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::Status { naos } => status(naos).await,
        Arguments::List { naos } => available_networks(naos).await,
        Arguments::Set { network, naos } => set(naos, network)
            .await
            .wrap_err("failed to execute set command")?,
    };

    Ok(())
}

async fn status(naos: Vec<NaoAddress>) {
    let results = join_all(naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        (
            nao_address,
            nao.get_network_status()
                .await
                .wrap_err_with(|| format!("failed to get network status from {nao_address}")),
        )
    }))
    .await;

    for (nao_address, status_result) in results {
        println!("{nao_address}");
        match status_result {
            Ok(status) => println!("{status}"),
            Err(error) => println!("{error:?}"),
        }
    }
}

async fn available_networks(naos: Vec<NaoAddress>) {
    let results = join_all(naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        (
            nao_address,
            nao.get_available_networks()
                .await
                .wrap_err_with(|| format!("failed to get available networks from {nao_address}")),
        )
    }))
    .await;

    for (nao_address, available_networks_result) in results {
        println!("{nao_address}");
        match available_networks_result {
            Ok(available_networks) => println!("{available_networks}"),
            Err(error) => println!("{error:?}"),
        }
    }
}

async fn set(naos: Vec<NaoAddress>, network: Network) -> Result<()> {
    let tasks = naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        nao.set_network(network)
            .await
            .wrap_err_with(|| format!("failed to set network on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some network setting tasks")?;

    Ok(())
}
