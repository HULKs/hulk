use std::path::{Path, PathBuf};

use color_eyre::{eyre::Context, Result};
use tokio::fs::{create_dir_all, rename};

use crate::download::{download_with_fallback, CONNECT_TIMEOUT};

/// Downloads the NAO image for a specified version.
///
/// This function ensures the NAO image is downloaded. If the image is already exists, it will
/// do nothing. If not, it will try to download it.
///
/// Returns the path to the downloaded image.
pub async fn download_image(version: &str, data_home: impl AsRef<Path>) -> Result<PathBuf> {
    let data_home = data_home.as_ref();
    let downloads_directory = data_home.join("image/");
    let image_name = format!("nao-image-HULKs-OS-{version}.ext3.gz.opn");
    let image_path = downloads_directory.join(&image_name);
    let download_path = image_path.with_extension("tmp");

    if image_path.exists() {
        return Ok(image_path);
    }

    create_dir_all(&downloads_directory)
        .await
        .wrap_err("failed to create download directory")?;

    let urls = [
        format!("http://bighulk.hulks.dev/image/{image_name}"),
        format!("https://github.com/HULKs/meta-nao/releases/download/{version}/{image_name}"),
    ];
    download_with_fallback(urls, &download_path, CONNECT_TIMEOUT)
        .await
        .wrap_err("failed to download image")?;

    rename(download_path, &image_path)
        .await
        .wrap_err("failed to rename image")?;

    Ok(image_path)
}
