use anyhow::Context;
use clap::Subcommand;
use futures::future::join_all;

use nao::{Nao, Network};
use repository::Repository;

use crate::{
    parsers::{parse_network, NaoAddress, NETWORK_POSSIBLE_VALUES},
    results::gather_results,
};

#[derive(Subcommand)]
pub enum Arguments {
    Status {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[clap(required = true)]
        naos: Vec<NaoAddress>,
    },
    AvailableNetworks {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[clap(required = true)]
        naos: Vec<NaoAddress>,
    },
    Set {
        /// The network to connect the wireless device to e.g. SPL_A or None (None disconnects from anything)
        #[clap(possible_values = NETWORK_POSSIBLE_VALUES, parse(try_from_str = parse_network))]
        network: Network,
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[clap(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub async fn wireless(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    match arguments {
        Arguments::Status { naos } => status(naos, repository).await,
        Arguments::AvailableNetworks { naos } => available_networks(naos, repository).await,
        Arguments::Set { network, naos } => set(naos, network, repository)
            .await
            .context("Failed to execute set command")?,
    };

    Ok(())
}

async fn status(naos: Vec<NaoAddress>, repository: &Repository) {
    let results = join_all(naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        (
            nao_address,
            nao.get_network_status()
                .await
                .with_context(|| format!("Failed to get network status from {nao_address}")),
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

async fn available_networks(naos: Vec<NaoAddress>, repository: &Repository) {
    let results = join_all(naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        (
            nao_address,
            nao.get_available_networks()
                .await
                .with_context(|| format!("Failed to get available networks from {nao_address}")),
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

async fn set(
    naos: Vec<NaoAddress>,
    network: Network,
    repository: &Repository,
) -> anyhow::Result<()> {
    let tasks = naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        nao.set_network(network)
            .await
            .with_context(|| format!("Failed to set network on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some network setting tasks")?;

    Ok(())
}
