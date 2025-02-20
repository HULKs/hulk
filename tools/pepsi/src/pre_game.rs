use std::path::Path;

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
    cargo::{self, build, cargo, environment::EnvironmentArguments, CargoCommand},
    deploy_config::DeployConfig,
    progress_indicator::ProgressIndicator,
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
    /// Prepare everything for the upload without performing the actual one
    #[arg(long)]
    pub prepare: bool,
}

pub async fn pre_game(arguments: Arguments, repository: &Repository) -> Result<()> {
    let config = DeployConfig::read_from_file(repository)
        .await
        .wrap_err("failed to read deploy config from file")?;
    let naos = config.naos();

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
        &naos,
        "Executing pregame tasks",
        |nao_address, progress_bar| async move {
            setup_nao(
                nao_address,
                upload_directory,
                arguments,
                config.wifi,
                progress_bar,
                repository,
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
    arguments: &PreGameArguments,
    wifi: Network,
    progress: ProgressBar,
    repository: &Repository,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::try_new_with_ping(nao_address.ip).await?;

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
        progress.set_message(format!("Uploading: {}", status))
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
