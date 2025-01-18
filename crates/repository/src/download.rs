use std::{ffi::OsStr, time::Duration};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::process::Command;

pub async fn download(url: impl AsRef<OsStr>, output_path: impl AsRef<OsStr>) -> Result<()> {
    let default_connect_timeout = Duration::from_secs(5);
    download_with_fallback_and_timeout(std::iter::once(url), output_path, default_connect_timeout)
        .await
}

/// Download a file from a list of URLs using `curl`.
///
/// This function takes a list of URLs to download from, a path to the output file,
/// and a connection timeout duration. It tries to download the file from each URL
/// in the list until it succeeds or runs out of URLs.
pub async fn download_with_fallback_and_timeout(
    urls: impl IntoIterator<Item = impl AsRef<OsStr>>,
    output_path: impl AsRef<OsStr>,
    connect_timeout: Duration,
) -> Result<()> {
    for url in urls.into_iter() {
        let url = url.as_ref();
        eprintln!("Downloading from {url:?}");

        let status = Command::new("curl")
            .arg("--connect-timeout")
            .arg(connect_timeout.as_secs_f32().to_string())
            .arg("--fail")
            .arg("--location")
            .arg("--progress-bar")
            .arg("--output")
            .arg(&output_path)
            .arg(url)
            .status()
            .await
            .wrap_err("failed to spawn command")?;

        if status.success() {
            return Ok(());
        }
    }

    bail!("failed to download from any URL");
}
