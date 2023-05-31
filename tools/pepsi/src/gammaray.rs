use std::path::PathBuf;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use constants::OS_VERSION;
use nao::Nao;
use repository::get_image_path;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

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

pub async fn gammaray(arguments: Arguments) -> Result<()> {
    let version = arguments.os_version.as_deref().unwrap_or(OS_VERSION);
    let image_path = match arguments.image_path {
        Some(image_path) => image_path,
        None => get_image_path(version).await?,
    };
    let image_path = image_path.as_path();

    ProgressIndicator::map_tasks(
        arguments.naos,
        "Uploading image...",
        |nao_address| async move {
            let nao = Nao::try_new_with_ping(nao_address.ip).await?;
            nao.flash_image(image_path)
                .await
                .wrap_err_with(|| format!("failed to flash image to {nao_address}"))
        },
    )
    .await;

    Ok(())
}
