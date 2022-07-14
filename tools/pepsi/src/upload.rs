use anyhow::Context;
use clap::Args;
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
    #[clap(long, default_value = "incremental")]
    pub profile: String,
    /// Do not update nor install SDK
    #[clap(long)]
    pub no_sdk_installation: bool,
    /// Do not build before uploading
    #[clap(long)]
    pub no_build: bool,
    /// Do not restart HULK nor HULA service after uploading
    #[clap(long)]
    pub no_restart: bool,
    /// Do not remove existing remote files during uploading
    #[clap(long)]
    pub no_clean: bool,
    /// Do not run aliveness (ignored if --no-restart given because it requires restarting HULA)
    #[clap(long)]
    pub no_aliveness: bool,
    /// Do not enable communication
    #[clap(long)]
    pub no_communication: bool,
    /// The NAOs to upload to e.g. 20w or 10.1.24.22
    #[clap(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn upload(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let nao_numbers = arguments
        .naos
        .iter()
        .map(|nao_address| (*nao_address).try_into())
        .collect::<Result<Vec<NaoNumber>, _>>()
        .context("Failed to convert NAO address into NAO numbers")?;

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
        .context("Failed to build the code")?;
    }

    communication(
        match arguments.no_communication {
            true => CommunicationArguments::Disable { nao_numbers },
            false => CommunicationArguments::Enable { nao_numbers },
        },
        repository,
    )
    .await
    .context("Failed to set communication enablement directory")?;

    let (_temporary_directory, hulk_directory) = repository
        .create_upload_directory(arguments.profile)
        .await
        .context("Failed to create upload directory")?;

    if !arguments.no_restart {
        println!("Stopping HULK");
        hulk(
            HulkArguments {
                action: SystemctlAction::Stop,
                naos: arguments.naos.clone(),
            },
            repository,
        )
        .await
        .context("Failed to stop HULK service")?;
    }

    let tasks = arguments.naos.iter().map(|nao_address| {
        let hulk_directory = hulk_directory.clone();
        async move {
            let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

            println!("Starting upload to {}", nao_address);
            nao.upload(hulk_directory, !arguments.no_clean)
                .await
                .with_context(|| format!("Failed to power {nao_address} off"))
        }
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some upload tasks")?;

    if !arguments.no_restart {
        let tasks = arguments.naos.iter().map(|nao_address| async move {
            let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

            nao.set_aliveness(!arguments.no_aliveness)
                .await
                .with_context(|| format!("Failed to set aliveness on {nao_address}"))
        });

        let results = join_all(tasks).await;
        gather_results(
            results,
            "Failed to execute some systemctl restart hula tasks",
        )?;

        hulk(
            HulkArguments {
                action: SystemctlAction::Start,
                naos: arguments.naos,
            },
            repository,
        )
        .await
        .context("Failed to start HULK service")?;
    }

    Ok(())
}
