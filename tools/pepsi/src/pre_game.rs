use std::{collections::HashMap, path::Path};

use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Args,
};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};

use argument_parsers::{
    parse_network, NaoAddress, NaoAddressPlayerAssignment, NETWORK_POSSIBLE_VALUES,
};
use indicatif::ProgressBar;
use log::warn;
use nao::{Nao, Network, SystemctlAction};
use repository::{
    communication::set_communication, configuration::get_os_version, location::set_location,
    recording::set_recording_intervals, upload::populate_upload_directory,
};
use tempfile::tempdir;

use crate::{
    cargo::{cargo, Arguments as CargoArguments},
    player_number::{player_number, Arguments as PlayerNumberArguments},
    progress_indicator::ProgressIndicator,
    recording::parse_key_value,
};

#[derive(Args)]
pub struct Arguments {
    #[command(flatten)]
    pub cargo: CargoArguments,
    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[arg(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[arg(long)]
    pub no_clean: bool,
    /// Skip the OS version check
    #[arg(long)]
    pub skip_os_check: bool,
    /// Enable communication, communication is disabled by default
    #[arg(long)]
    pub with_communication: bool,
    /// Intervals between cycle recordings, e.g. Control=1,VisionTop=30 to record every cycle in Control
    /// and one out of every 30 in VisionTop. Set to 0 or don't specify to disable recording for a cycler.
    #[arg(
        long,
        value_delimiter=',',
        value_parser = parse_key_value::<String, usize>,
        default_value = "Control=1,VisionTop=30,VisionBottom=30,SplNetwork=1",
    )]
    pub recording_intervals: Vec<(String, usize)>,
    /// Prepare everything for the upload without performing the actual one
    #[arg(long)]
    pub prepare: bool,
    /// The location to use for parameters
    pub location: String,
    /// The network to connect the wifi device to (None disconnects from anything)
    #[arg(
        value_parser = PossibleValuesParser::new(NETWORK_POSSIBLE_VALUES)
            .map(|s| parse_network(&s).unwrap()))
    ]
    pub wifi: Network,
    /// The NAOs to upload to with player number assignments e.g. 20w:2 or 10.1.24.22:5 (player numbers start from 1)
    #[arg(required = true)]
    pub assignments: Vec<NaoAddressPlayerAssignment>,
}

pub async fn pre_game(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    let repository_root = repository_root.as_ref();

    let naos: Vec<_> = arguments
        .assignments
        .iter()
        .map(|assignment| assignment.nao_address)
        .collect();

    set_recording_intervals(
        HashMap::from_iter(arguments.recording_intervals.clone()),
        repository_root,
    )
    .await
    .wrap_err("failed to set recording settings")?;

    set_location("nao", &arguments.location, repository_root)
        .await
        .wrap_err_with(|| format!("failed setting location for nao to {}", arguments.location))?;

    set_communication(arguments.with_communication, repository_root)
        .await
        .wrap_err("failed to set communication")?;

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
        repository_root
    )
    .await
    .wrap_err("failed to set player numbers")?;

    if !arguments.no_build {
        cargo("build", arguments.cargo.clone(), repository_root)
            .await
            .wrap_err("failed to build the code")?;
    }

    if arguments.prepare {
        warn!("Preparation complete, skipping the rest");
        return Ok(());
    }

    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    populate_upload_directory(&upload_directory, &arguments.cargo.profile, repository_root)
        .await
        .wrap_err("failed to populate upload directory")?;

    let arguments = &arguments;
    let upload_directory = &upload_directory;

    ProgressIndicator::map_tasks(
        &naos,
        "Executing pregame tasks",
        |nao_address, progress_bar| async move {
            setup_nao(
                nao_address,
                upload_directory,
                arguments,
                progress_bar,
                repository_root,
            )
            .await
        },
    )
    .await;

    Ok(())
}

async fn setup_nao(
    nao_address: &NaoAddress,
    upload_directory: impl AsRef<Path>,
    arguments: &Arguments,
    progress: ProgressBar,
    repository_root: &Path,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::try_new_with_ping(nao_address.ip).await?;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let nao_os_version = nao
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {nao_address}"))?;
        let expected_os_version = get_os_version(repository_root)
            .await
            .wrap_err("failed to get configured OS version")?;
        if nao_os_version != expected_os_version {
            bail!("mismatched OS versions: Expected {expected_os_version}, found {nao_os_version}");
        }
    }

    progress.set_message("Stopping HULK...");
    nao.execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .wrap_err_with(|| format!("failed to stop HULK service on {nao_address}"))?;

    progress.set_message("Uploading: ...");
    nao.upload(upload_directory, "hulk", !arguments.no_clean, |status| {
        progress.set_message(format!("Uploading: {}", status))
    })
    .await
    .wrap_err_with(|| format!("failed to upload binary to {nao_address}"))?;

    if arguments.wifi != Network::None {
        progress.set_message("Scanning for WiFi...");
        nao.scan_networks()
            .await
            .wrap_err_with(|| format!("failed to scan for networks on {nao_address}"))?;
    }

    progress.set_message("Setting WiFi...");
    nao.set_wifi(arguments.wifi)
        .await
        .wrap_err_with(|| format!("failed to set network on {nao_address}"))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        if let Err(error) = nao.execute_systemctl(SystemctlAction::Start, "hulk").await {
            let logs = nao
                .retrieve_logs()
                .await
                .wrap_err("failed to retrieve logs")?;
            bail!("failed to restart hulk: {error:#?}\nLogs:\n{logs}")
        };
    }

    Ok(())
}
