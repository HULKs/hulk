use std::path::Path;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;
use nao::{Nao, SystemctlAction};
use repository::Repository;

use crate::{
    cargo::{cargo, Arguments as CargoArguments, Command},
    communication::{communication, Arguments as CommunicationArguments},
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
    nao: Nao,
    hulk_directory: impl AsRef<Path>,
    progress: &Task,
    arguments: &Arguments,
) -> Result<()> {
    if !arguments.skip_os_check && !nao.has_stable_os_version().await {
        return Ok(());
    }

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
    let nao_numbers = arguments
        .naos
        .iter()
        .map(|nao_address| (*nao_address).try_into())
        .collect::<Result<Vec<NaoNumber>, _>>()
        .wrap_err("failed to convert NAO address into NAO numbers")?;

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

    communication(
        match arguments.no_communication {
            true => CommunicationArguments::Disable { nao_numbers },
            false => CommunicationArguments::Enable { nao_numbers },
        },
        repository,
    )
    .await
    .wrap_err("failed to set communication enablement directory")?;

    let (_temporary_directory, hulk_directory) = repository
        .create_upload_directory(arguments.profile.as_str())
        .await
        .wrap_err("failed to create upload directory")?;

    let multi_progress = ProgressIndicator::new();

    let tasks = arguments.naos.iter().map(|nao_address| {
        let hulk_directory = hulk_directory.clone();
        let arguments = &arguments;
        let multi_progress = multi_progress.clone();

        async move {
            let progress = multi_progress.task(nao_address.to_string());
            let nao = Nao::new(nao_address.ip);

            progress
                .finish_with(upload_with_progress(nao, hulk_directory, &progress, arguments).await)
        }
    });

    join_all(tasks).await;

    Ok(())
}
