use anyhow::Context;
use nao::Network;
use repository::Repository;
use structopt::StructOpt;

use crate::{
    parsers::{parse_network, NaoAddressPlayerAssignment, NETWORK_POSSIBLE_VALUES},
    player_number::{player_number, Arguments as PlayerNumberArguments},
    upload::{upload, Arguments as UploadArguments},
    wireless::{wireless, Arguments as WirelessArguments},
};

#[derive(StructOpt)]
pub struct Arguments {
    #[structopt(long, default_value = "release")]
    pub profile: String,
    /// Do not update nor install SDK
    #[structopt(long)]
    pub no_sdk_installation: bool,
    /// Do not build before uploading
    #[structopt(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[structopt(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[structopt(long)]
    pub no_clean: bool,
    /// Enable aliveness (ignored if --no-restart given because it requires restarting HULA)
    #[structopt(long)]
    pub with_aliveness: bool,
    /// Enable communication
    #[structopt(long)]
    pub with_communication: bool,
    /// The network to connect the wireless device to e.g. SPL_A or None (None disconnects from anything)
    #[structopt(possible_values = NETWORK_POSSIBLE_VALUES, parse(try_from_str = parse_network))]
    network: Network,
    /// The NAOs to upload to with player number assignments e.g. 20w:2 or 10.1.24.22:5 (player numbers start from 1)
    #[structopt(required = true)]
    pub assignments: Vec<NaoAddressPlayerAssignment>,
}

pub async fn pre_game(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let naos: Vec<_> = arguments
        .assignments
        .iter()
        .map(|assignment| assignment.nao_address)
        .collect();

    player_number(
        PlayerNumberArguments {
            assignments: arguments
                .assignments
                .iter()
                .copied()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .context("Failed to convert NAO address assignments into NAO number assignments for player number setting")?
        },
        repository
    )
    .await
    .context("Failed to set player numbers")?;

    upload(
        UploadArguments {
            profile: arguments.profile,
            no_sdk_installation: arguments.no_sdk_installation,
            no_build: arguments.no_build,
            no_restart: arguments.no_restart,
            no_clean: arguments.no_clean,
            no_aliveness: !arguments.with_aliveness,
            no_communication: !arguments.with_communication,
            naos: naos.clone(),
        },
        repository,
    )
    .await
    .context("Failed to upload")?;

    wireless(
        WirelessArguments::Set {
            network: arguments.network,
            naos,
        },
        repository,
    )
    .await
    .context("Failed to set wireless network")?;

    Ok(())
}
