use std::path::PathBuf;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use argument_parsers::NaoAddress;
use constants::OS_VERSION;
use nao::Nao;
use opn::verify_image;
use repository::{get_image_path, Repository};

use crate::progress_indicator::ProgressIndicator;

#[derive(Args)]
pub struct Arguments {
    /// Alternative path to an image
    #[arg(long)]
    image_path: Option<PathBuf>,
    /// Alternative HULKs-OS version e.g. 3.3
    #[arg(long)]
    os_version: Option<String>,
    /// The NAOs to flash the image to, e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    naos: Vec<NaoAddress>,
}

pub async fn gammaray(arguments: Arguments, repository: &Repository) -> Result<()> {
    let version = arguments.os_version.as_deref().unwrap_or(OS_VERSION);
    let image_path = match arguments.image_path {
        Some(image_path) => image_path,
        None => get_image_path(version).await?,
    };
    let image_path = image_path.as_path();

    verify_image(image_path).wrap_err("image verification failed")?;

    let hardware_ids = &repository.parameters_root().join("hardware_ids.json");

    ProgressIndicator::map_tasks(
        arguments.naos,
        "Uploading image ...",
        |nao_address, progress_bar| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            nao.flash_image(image_path, |msg| {
                progress_bar.set_message(format!("Uploading image: {}", msg))
            })
            .await
            .wrap_err_with(|| format!("failed to flash image to {nao_address}"))?;
            progress_bar.set_message("Uploading hardware ids...");
            nao.rsync_with_nao(false)
                .arg(hardware_ids.to_str().unwrap())
                .arg(format!("{}:/media/internal/", nao.host))
                .spawn()
                .wrap_err("failed to upload hardware ids")?;
            nao.reboot()
                .await
                .wrap_err_with(|| format!("failed to reboot {nao_address}"))
        },
    )
    .await;

    Ok(())
}
