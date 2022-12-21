use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Subcommand,
};
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::{Nao, Network};

use crate::{
    parsers::{parse_network, NaoAddress, NETWORK_POSSIBLE_VALUES},
    progress_indicator::ProgressIndicator,
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
    let multi_progress = ProgressIndicator::new();

    join_all(naos.into_iter().map(|nao_address| {
        let multi_progress = multi_progress.clone();
        async move {
            let progress = multi_progress.task(nao_address.to_string());
            let nao = Nao::new(nao_address.ip);

            progress.set_message("Retrieving network status...");
            progress.finish_with(
                nao.get_network_status()
                    .await
                    .wrap_err_with(|| format!("failed to get network status from {nao_address}")),
            );
        }
    }))
    .await;
}

async fn available_networks(naos: Vec<NaoAddress>) {
    let multi_progress = ProgressIndicator::new();

    let tasks =
        naos.into_iter().map(|nao_address| {
            let multi_progress = multi_progress.clone();
            async move {
                let progress = multi_progress.task(nao_address.to_string());
                let nao = Nao::new(nao_address.ip);

                progress.set_message("Retrieving available networks...");
                progress.finish_with(nao.get_available_networks().await.wrap_err_with(|| {
                    format!("failed to get available networks from {nao_address}")
                }));
            }
        });

    join_all(tasks).await;
}

async fn set(naos: Vec<NaoAddress>, network: Network) -> Result<()> {
    let multi_progress = ProgressIndicator::new();

    let tasks = naos.into_iter().map(|nao_address| {
        let multi_progress = multi_progress.clone();
        async move {
            let progress = multi_progress.task(nao_address.to_string());
            let nao = Nao::new(nao_address.ip);

            progress.set_message("Retrieving available networks...");
            progress.finish_with(
                nao.set_network(network)
                    .await
                    .wrap_err_with(|| format!("failed to set network on {nao_address}")),
            )
        }
    });

    join_all(tasks).await;

    Ok(())
}
