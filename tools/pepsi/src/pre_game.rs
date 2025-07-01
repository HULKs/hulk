use std::{collections::HashMap, path::Path};

use clap::Args;
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};

use argument_parsers::NaoAddress;
use indicatif::ProgressBar;
use nao::{Nao, Network, SystemctlAction};
use repository::{upload::get_hulk_binary, Repository};
use tempfile::tempdir;

use crate::{
    cargo::{self, build, cargo, environment::EnvironmentArguments, run, CargoCommand},
    deploy_config::DeployConfig,
    progress_indicator::ProgressIndicator,
    recording::parse_key_value,
};

#[derive(Args)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub pre_game: PreGameArguments,
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct PreGameArguments {
    /// Skip running the parameter tester
    #[arg(long)]
    pub skip_parameter_check: bool,
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
    )]
    pub recording_intervals: Option<Vec<(String, usize)>>,
    /// Prepare everything for the upload without performing the actual one
    #[arg(long)]
    pub prepare: bool,
    /// The NAOs to apply the pregame to, queried from the deploy.toml if not specified
    pub naos: Option<Vec<NaoAddress>>,
}

pub async fn pre_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    if !arguments.pre_game.skip_parameter_check {
        run_parameter_tester(arguments.environment.clone(), repository).await?;
    }

    let mut config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;

    config.with_communication |= arguments.pre_game.with_communication;
    if let Some(recording_intervals) = &arguments.pre_game.recording_intervals {
        config.recording_intervals = HashMap::from_iter(recording_intervals.iter().cloned());
    }

    let playing_naos = config.playing_naos()?;
    let naos = if let Some(naos) = &arguments.pre_game.naos {
        for nao in naos {
            if !playing_naos.contains(nao) {
                bail!("NAO with IP {nao} is not one of the playing NAOs in the deploy.toml");
            }
        }
        naos
    } else {
        &playing_naos
    };
    let wifi = config.wifi;

    config
        .configure_repository(repository)
        .await
        .wrap_err("failed to configure repository")?;

    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    let hulk_binary = get_hulk_binary(arguments.build.profile());

    let cargo_arguments = cargo::Arguments {
        manifest: Some(
            repository
                .root
                .join("crates/hulk_nao/Cargo.toml")
                .into_os_string(),
        ),
        environment: arguments.environment,
        cargo: arguments.build,
    };

    if !arguments.pre_game.no_build {
        cargo(cargo_arguments, repository, &[&hulk_binary])
            .await
            .wrap_err("failed to build")?;
    }
    if arguments.pre_game.prepare {
        eprintln!("Preparation complete, skipping the rest");
        return Ok(());
    }

    repository
        .populate_upload_directory(&upload_directory, hulk_binary)
        .await
        .wrap_err("failed to populate upload directory")?;

    let arguments = &arguments.pre_game;
    let upload_directory = &upload_directory;

    ProgressIndicator::map_tasks(
        naos,
        "Executing pregame tasks",
        |nao_address, progress_bar| async move {
            setup_nao(
                nao_address,
                upload_directory,
                arguments,
                wifi,
                progress_bar,
                repository,
            )
            .await
        },
    )
    .await;

    Ok(())
}

async fn run_parameter_tester(
    environment: EnvironmentArguments,
    repository: &Repository,
) -> Result<()> {
    let cargo_arguments = cargo::Arguments {
        manifest: Some(
            repository
                .root
                .join("tools/parameter_tester/Cargo.toml")
                .into_os_string(),
        ),
        environment,
        cargo: run::Arguments::default(),
    };

    cargo(cargo_arguments, repository, &[] as &[&str])
        .await
        .wrap_err("failed to run parameter tester")
}

async fn setup_nao(
    nao_address: &NaoAddress,
    upload_directory: impl AsRef<Path>,
    arguments: &PreGameArguments,
    wifi: Network,
    progress: ProgressBar,
    repository: &Repository,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::ping_until_available(nao_address.ip).await;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let nao_os_version = nao
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {nao_address}"))?;
        let expected_os_version = repository
            .read_os_version()
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
        progress.set_message(format!("Uploading: {status}"))
    })
    .await
    .wrap_err_with(|| format!("failed to upload binary to {nao_address}"))?;

    if wifi != Network::None {
        progress.set_message("Scanning for WiFi...");
        nao.scan_networks()
            .await
            .wrap_err_with(|| format!("failed to scan for networks on {nao_address}"))?;
    }

    progress.set_message("Setting WiFi...");
    nao.set_wifi(wifi)
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
