use std::path::Path;

use argument_parsers::NaoAddress;
use clap::Args;
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use nao::{Nao, SystemctlAction};
use repository::{configuration::read_os_version, upload::populate_upload_directory};
use tempfile::tempdir;

use crate::{
    cargo::{self, build, cargo, environment::EnvironmentArguments, CargoCommand},
    progress_indicator::{ProgressIndicator, Task},
};

#[derive(Args)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub upload: UploadArguments,
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten)]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct UploadArguments {
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
    /// The NAOs to upload to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

async fn upload_with_progress(
    nao_address: &NaoAddress,
    upload_directory: impl AsRef<Path>,
    arguments: &UploadArguments,
    progress: &Task,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    progress.set_message("Pinging NAO...");
    let nao = Nao::try_new_with_ping(nao_address.ip).await?;

    if !arguments.skip_os_check {
        progress.set_message("Checking OS version...");
        let nao_os_version = nao
            .get_os_version()
            .await
            .wrap_err_with(|| format!("failed to get OS version of {nao_address}"))?;
        let expected_os_version = read_os_version(repository_root)
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

pub async fn upload(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    let profile = arguments.build.profile().to_owned();

    let cargo_arguments = cargo::Arguments {
        manifest: Some(
            repository_root
                .as_ref()
                .join("crates/hulk_nao/Cargo.toml")
                .into_os_string(),
        ),
        environment: arguments.environment,
        cargo: arguments.build,
    };

    if !arguments.upload.no_build {
        cargo(cargo_arguments, &repository_root)
            .await
            .wrap_err("failed to build")?;
    }

    populate_upload_directory(&upload_directory, &profile, &repository_root)
        .await
        .wrap_err("failed to populate upload directory")?;

    let upload_arguments = &arguments.upload;
    let repository_root = &repository_root;
    let upload_directory = &upload_directory;

    let multi_progress = ProgressIndicator::new();
    arguments
        .upload
        .naos
        .iter()
        .map(|nao_address| {
            let progress = multi_progress.task(nao_address.to_string());
            progress.enable_steady_tick();
            async move {
                progress.finish_with(
                    upload_with_progress(
                        nao_address,
                        upload_directory,
                        upload_arguments,
                        &progress,
                        repository_root,
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
