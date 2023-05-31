use std::{collections::HashMap, path::Path};

use clap::Args;
use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use constants::OS_VERSION;
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
    /// Skip the OS version check
    #[arg(long)]
    pub skip_os_check: bool,
    /// The NAOs to upload to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

fn get_head_id<'a>(
    nao_address: &NaoAddress,
    hardware_ids: &'a HashMap<u8, HardwareIds>,
) -> Result<&'a str> {
    let nao_number: NaoNumber = (*nao_address)
        .try_into()
        .wrap_err("failed to convert NAO address into NAO numbers")?;
    let head_id = &hardware_ids
        .get(&nao_number.number)
        .ok_or_else(|| eyre!("no hardware ID found for {}", nao_number.number))?
        .head_id;

    Ok(head_id)
}

async fn upload_with_progress(
    nao_address: &NaoAddress,
    head_id: &str,
    hulk_directory: impl AsRef<Path>,
    repository: &Repository,
    arguments: &Arguments,
    progress: &Task,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::try_new_with_ping(nao_address.ip).await?;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let os_version = nao
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {nao_address}"))?;
        if os_version != OS_VERSION {
            bail!("mismatched OS versions: Expected {OS_VERSION}, found {os_version}");
        }
    }

    progress.set_message("Setting communication...");
    repository
        .set_communication(head_id, !arguments.no_communication)
        .await
        .wrap_err_with(|| format!("failed to set communication enablement for {head_id}"))?;

    progress.set_message("Stopping HULK...");
    nao.execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .wrap_err_with(|| format!("failed to stop HULK service on {nao_address}"))?;

    progress.set_message("Uploading...");
    nao.upload(hulk_directory, !arguments.no_clean)
        .await
        .wrap_err_with(|| format!("failed to upload binary to {nao_address}"))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        nao.execute_systemctl(SystemctlAction::Start, "hulk")
            .await
            .wrap_err_with(|| format!("failed to stop HULK service on {nao_address}"))?;
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
        .map(|nao_address| (nao_address, multi_progress.task(nao_address.to_string())))
        .map(|(nao_address, progress)| {
            let arguments = &arguments;
            let head_id = get_head_id(nao_address, &hardware_ids);
            let hulk_directory = hulk_directory.clone();

            progress.enable_steady_tick();
            async move {
                match head_id {
                    Ok(head_id) => progress.finish_with(
                        upload_with_progress(
                            nao_address,
                            head_id,
                            hulk_directory,
                            repository,
                            arguments,
                            &progress,
                        )
                        .await,
                    ),
                    Err(message) => progress.finish_with_error(message),
                }
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
