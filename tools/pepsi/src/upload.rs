use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::{Nao, SystemctlAction};
use repository::Repository;

use crate::{
    cargo::{cargo, Arguments as CargoArguments, Command},
    communication::{communication, Arguments as CommunicationArguments},
    hulk::{hulk, Arguments as HulkArguments},
    parsers::{NaoAddress, NaoNumber},
    results::gather_results,
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
        .create_upload_directory(arguments.profile)
        .await
        .wrap_err("failed to create upload directory")?;

    if !arguments.no_restart {
        println!("Stopping HULK");
        hulk(HulkArguments {
            action: SystemctlAction::Stop,
            naos: arguments.naos.clone(),
        })
        .await
        .wrap_err("failed to stop HULK service")?;
    }

    let tasks = arguments.naos.iter().map(|nao_address| {
        let hulk_directory = hulk_directory.clone();
        async move {
            let nao = Nao::new(nao_address.ip);
            println!("Starting upload to {nao_address}");
            nao.upload(hulk_directory, !arguments.no_clean)
                .await
                .wrap_err_with(|| format!("failed to power {nao_address} off"))
        }
    });

    let results = join_all(tasks).await;
    gather_results(results, "failed to execute some upload tasks")?;

    if !arguments.no_restart {
        hulk(HulkArguments {
            action: SystemctlAction::Start,
            naos: arguments.naos,
        })
        .await
        .wrap_err("failed to start HULK service")?;
    }

    Ok(())
}
