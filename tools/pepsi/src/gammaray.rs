use std::path::PathBuf;

use clap::Args;
use color_eyre::{eyre::Context, Result};
use constants::OS_VERSION;
use futures::{stream::FuturesUnordered, StreamExt};
use nao::Nao;
use repository::get_image_path;

use crate::{parsers::NaoAddress, results::gather_results};

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

    let results: Vec<_> = arguments
        .naos
        .into_iter()
        .map(|nao_address| async move {
            let nao = Nao::new(nao_address.ip);
            println!("Starting image upload to {nao_address}");
            nao.flash_image(image_path)
                .await
                .wrap_err_with(|| format!("failed to flash image to {nao_address}"))
        })
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;

    gather_results(results, "failed to execute some image flashing tasks")
}
