use std::{path::Path, time::Duration};

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use nao::{Nao, SystemctlAction};
use repository::Repository;

use crate::{
    cargo::{cargo, Arguments as CargoArguments, Command},
    communication::{communication, Arguments as CommunicationArguments},
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
    #[arg(long)]
    pub skip_os_check: bool,
}

async fn upload_with_progress(
    nao: Nao,
    hulk_directory: impl AsRef<Path>,
    progress: &ProgressBar,
    arguments: &Arguments,
) -> anyhow::Result<()> {
    progress.set_message("Stopping HULK...");
    nao.execute_systemctl(SystemctlAction::Stop, "hulk")
        .await
        .with_context(|| format!("failed to stop HULK service on {}", nao.host()))?;

    progress.set_message("Uploading...");
    nao.upload(hulk_directory, !arguments.no_clean)
        .await
        .with_context(|| format!("failed to power {} off", nao.host()))?;

    if !arguments.no_restart {
        progress.set_message("Restarting HULK...");
        nao.execute_systemctl(SystemctlAction::Start, "hulk")
            .await
            .with_context(|| format!("failed to stop HULK service on {}", nao.host()))?;
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

    let multi_progress = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
    let spinner_error_style =
        ProgressStyle::with_template("{prefix:.bold.dim} {wide_msg:.red}").unwrap();

    let spinner_success_style =
        ProgressStyle::with_template("{prefix:.bold.dim} {wide_msg:.green}").unwrap();

    let tasks = arguments.naos.iter().map(|nao_address| {
        let hulk_directory = hulk_directory.clone();
        let multi_progress = multi_progress.clone();
        let arguments = &arguments;
        let spinner_style = spinner_style.clone();
        let spinner_error_style = spinner_error_style.clone();
        let spinner_success_style = spinner_success_style.clone();

        async move {
            let spinner = ProgressBar::new(10)
                .with_style(spinner_style)
                .with_prefix(nao_address.to_string());
            spinner.enable_steady_tick(Duration::from_millis(200));
            let progress = multi_progress.add(spinner);
            let nao = Nao::new(nao_address.ip);

            if !arguments.skip_os_check && !nao.has_stable_os_version().await {
                return Ok(());
            }

            match upload_with_progress(nao, hulk_directory, &progress, arguments).await {
                Ok(_) => {
                    progress.set_style(spinner_success_style);
                    progress.finish_with_message("✓ Done")
                }
                Err(error) => {
                    progress.set_style(spinner_error_style);
                    progress.finish_with_message(format!("✗ {error}"));
                }
            }
            Ok(())
        }
    });

    let results = join_all(tasks).await;
    gather_results(results, "failed to execute some upload tasks")?;

    Ok(())
}
