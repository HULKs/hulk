use std::path::PathBuf;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::NaoAddress;
use nao::Nao;
use opn::verify_image;
use repository::{data_home::get_data_home, image::download_image, Repository};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Alternative path to an image
    #[arg(long)]
    image_path: Option<PathBuf>,
    /// Alternative HULKs-OS version e.g. 3.3
    #[arg(long)]
    version: Option<String>,
    /// The NAOs to flash the image to, e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    naos: Vec<NaoAddress>,
}

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    let version = match arguments.version {
        Some(version) => version,
        None => repository
            .read_os_version()
            .await
            .wrap_err("failed to get OS version")?,
    };
    let data_home = get_data_home()?;
    let image_path = match arguments.image_path {
        Some(image_path) => image_path,
        None => download_image(&version, data_home).await?,
    };
    let image_path = image_path.as_path();

    verify_image(image_path).wrap_err("image verification failed")?;

    let team_toml = &repository.root.join("etc/parameters/team.toml");

    // prevent moving String into async closure
    let version = &version;

    ProgressIndicator::map_tasks(
        arguments.naos,
        format!("Uploading image v{version}: ..."),
        |nao_address, progress_bar| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            nao.flash_image(image_path, |msg| {
                progress_bar.set_message(format!("Uploading image v{version}: {}", msg))
            })
            .await
            .wrap_err_with(|| format!("failed to flash image to {nao_address}"))?;
            progress_bar.set_message("Uploading team configuration...");
            nao.rsync_with_nao()?
                .arg(team_toml)
                .arg(format!("{}:/media/internal/", nao.address))
                .spawn()
                .wrap_err("failed to upload team configuration")?;
            nao.reboot()
                .await
                .wrap_err_with(|| format!("failed to reboot {nao_address}"))
        },
    )
    .await;

    Ok(())
}
