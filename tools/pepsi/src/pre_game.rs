use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::{parse_network, NaoAddressPlayerAssignment, NETWORK_POSSIBLE_VALUES};
use nao::Network;
use repository::Repository;

use crate::{
    player_number::{player_number, Arguments as PlayerNumberArguments},
    recording::{parse_key_value, recording, Arguments as RecordingArguments},
    upload::{upload, Arguments as UploadArguments},
    wireless::{wireless, Arguments as WirelessArguments},
};

#[derive(Args)]
pub struct Arguments {
    #[arg(long, default_value = "release")]
    pub profile: String,
    /// Do not update nor install SDK
    #[arg(long)]
    pub no_sdk_installation: bool,
    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[arg(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[arg(long)]
    pub no_clean: bool,
    /// Enable communication
    #[arg(long)]
    pub with_communication: bool,
    /// Skip the OS version check
    #[arg(long)]
    pub skip_os_check: bool,
    /// Intervals between cycle recordings, e.g. Control=1,VisionTop=30 to record every cycle in Control
    /// and one out of every 30 in VisionTop. Set to 0 or don't specify to disable recording for a cycler.
    #[arg(long, value_delimiter=',', value_parser = parse_key_value::<String, usize>, default_value = "Control=1,VisionTop=30,VisionBottom=30")]
    pub recording_intervals: Vec<(String, usize)>,
    /// The location to use for parameters
    pub location: String,
    /// The network to connect the wireless device to (None disconnects from anything)
    #[arg(
        value_parser = PossibleValuesParser::new(NETWORK_POSSIBLE_VALUES)
            .map(|s| parse_network(&s).unwrap()))
    ]
    pub network: Network,
    /// The NAOs to upload to with player number assignments e.g. 20w:2 or 10.1.24.22:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoAddressPlayerAssignment>,
    /// Use a remote machine for compilation, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}

pub async fn pre_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let naos: Vec<_> = arguments
        .assignments
        .iter()
        .map(|assignment| assignment.nao_address)
        .collect();

    recording(
        RecordingArguments {
            recording_intervals: arguments.recording_intervals,
        },
        repository,
    )
    .await
    .wrap_err("failed to set cyclers to be recorded")?;

    repository
        .set_location("nao", &arguments.location)
        .await
        .wrap_err_with(|| format!("failed setting location for nao to {}", arguments.location))?;

    player_number(
        PlayerNumberArguments {
            assignments: arguments
                .assignments
                .iter()
                .copied()
                .map(TryFrom::try_from)
                .collect::<Result<Vec<_>, _>>()
                .wrap_err("failed to convert NAO address assignments into NAO number assignments for player number setting")?
        },
        repository
    )
    .await
    .wrap_err("failed to set player numbers")?;

    wireless(WirelessArguments::Set {
        network: arguments.network,
        naos: naos.clone(),
    })
    .await
    .wrap_err("failed to set wireless network")?;

    upload(
        UploadArguments {
            profile: arguments.profile,
            no_sdk_installation: arguments.no_sdk_installation,
            no_build: arguments.no_build,
            no_restart: arguments.no_restart,
            no_clean: arguments.no_clean,
            no_communication: !arguments.with_communication,
            skip_os_check: arguments.skip_os_check,
            naos: naos,
            remote: arguments.remote,
        },
        repository,
    )
    .await
    .wrap_err("failed to upload")?;

    Ok(())
}
