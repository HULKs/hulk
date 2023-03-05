use std::{collections::HashMap, path::Path};

use clap::Args;
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use nao::{Nao, SystemctlAction};
use repository::{HardwareIds, Repository};

use crate::{
    cargo::{cargo, Arguments as CargoArguments, Command},
    parsers::{NaoAddress, NaoNumber},
    progress_indicator::{ProgressIndicator, Task},
};

#[derive(Args)]
pub struct Arguments {
    #[arg(long, default_value = "incremental")]
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
    /// Do not enable communication
    #[arg(long)]
    pub no_communication: bool,
    /// The NAOs to upload to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
    #[arg(long)]
    pub skip_os_check: bool,
}

async fn upload_with_progress(
    nao_address: &NaoAddress,
    hardware_ids: HashMap<u8, HardwareIds>,
    repository: &Repository,
    hulk_directory: impl AsRef<Path>,
    progress: &Task,
    arguments: &Arguments,
) -> Result<()> {
    let nao_number: NaoNumber = (*nao_address)
        .try_into()
        .wrap_err("failed to convert NAO address into NAO numbers")?;
    let nao = Nao::new(nao_address.ip);
    let head_id = &hardware_ids
        .get(&nao_number.number)
        .ok_or_else(|| eyre!("no hardware ID found for {}", nao_number.number))?
        .head_id;

    if !arguments.skip_os_check && !nao.has_stable_os_version().await {
        return Ok(());
    }

    progress.set_message("Setting communication...");
    repository
        .set_communication(head_id, !arguments.no_communication)
        .await
        .wrap_err_with(|| format!("failed to set communication enablement for {nao_number}"))?;

    progress.set_message("Stopping HULK...");
    nao.execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .wrap_err_with(|| format!("failed to stop HULK service on {}", nao.host()))?;

    progress.set_message("Uploading...");
    nao.upload(hulk_directory, !arguments.no_clean)
        .await
        .wrap_err_with(|| format!("failed to power {} off", nao.host()))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        nao.execute_systemctl(SystemctlAction::Start, "hulk")
            .await
            .wrap_err_with(|| format!("failed to stop HULK service on {}", nao.host()))?;
    }
    Ok(())
}

pub async fn upload(arguments: Arguments, repository: &Repository) -> Result<()> {
    if !arguments.no_build {
        cargo(
            CargoArguments {
                workspace: false,
                profile: arguments.profile.clone(),
                target: "nao".to_string(),
                no_sdk_installation: arguments.no_sdk_installation,
                passthrough_arguments: Vec::new(),
            },
            repository,
            Command::Build,
        )
        .await
        .wrap_err("failed to build the code")?;
    }

    let (_temporary_directory, hulk_directory) = repository
        .create_upload_directory(arguments.profile.as_str())
        .await
        .wrap_err("failed to create upload directory")?;

    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .wrap_err("failed to get hardware IDs")?;

    let multi_progress = ProgressIndicator::new();

    arguments
        .naos
        .iter()
        .map(|nao_address| {
            let arguments = &arguments;
            let repository = &repository;
            let hulk_directory = hulk_directory.clone();
            let hardware_ids = hardware_ids.clone();
            let progress = multi_progress.task(nao_address.to_string());
            async move {
                progress.finish_with(
                    upload_with_progress(
                        nao_address,
                        hardware_ids,
                        repository,
                        hulk_directory,
                        &progress,
                        arguments,
                    )
                    .await,
                )
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
