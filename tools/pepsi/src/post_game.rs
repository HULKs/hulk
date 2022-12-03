use std::path::PathBuf;

use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};

use nao::{Network, SystemctlAction};

use crate::{
    hulk::{hulk, Arguments as HulkArguments},
    logs::{logs, Arguments as LogsArguments},
    parsers::{parse_network, NaoAddress, NETWORK_POSSIBLE_VALUES},
    wireless::{wireless, Arguments as WirelessArguments},
};

#[derive(Args)]
pub struct Arguments {
    /// The network to connect the wireless device to (None disconnects from anything)
    #[arg(
        value_parser = PossibleValuesParser::new(NETWORK_POSSIBLE_VALUES)
            .map(|s| parse_network(&s).unwrap()))
    ]
    pub network: Network,
    /// Directory where to store the downloaded logs (will be created if not existing)
    pub log_directory: PathBuf,
    /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn post_game(arguments: Arguments) -> Result<()> {
    hulk(HulkArguments {
        action: SystemctlAction::Stop,
        naos: arguments.naos.clone(),
    })
    .await
    .wrap_err("failed to start HULK service")?;

    logs(LogsArguments::Download {
        log_directory: arguments.log_directory,
        naos: arguments.naos.clone(),
    })
    .await
    .wrap_err("failed to download logs")?;

    wireless(WirelessArguments::Set {
        network: arguments.network,
        naos: arguments.naos,
    })
    .await
    .wrap_err("failed to set wireless network")?;

    Ok(())
}
