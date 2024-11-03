use std::path::Path;

use color_eyre::{eyre::Context, Result};
use tokio::fs::{remove_file, symlink};

/// Create or replace a symlink from the source path to the destination path.
///
/// If a symlink already exists at the destination path, it is removed before creating the new symlink.
/// If the symlink cannot be created, an error is returned.
pub async fn create_symlink(source: &Path, destination: &Path) -> Result<()> {
    if destination.read_link().is_ok() {
        remove_file(&destination)
            .await
            .wrap_err("failed to remove current symlink")?;
    }
    symlink(&source, &destination)
        .await
        .wrap_err("failed to create symlink")?;
    Ok(())
}
