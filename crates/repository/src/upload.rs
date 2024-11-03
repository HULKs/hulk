use std::path::Path;

use color_eyre::{eyre::Context, Result};
use tokio::fs::{create_dir_all, symlink};

pub async fn populate_upload_directory(
    upload_directory: impl AsRef<Path>,
    profile: &str,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let upload_directory = upload_directory.as_ref();
    let repository_root = repository_root.as_ref();

    // the target directory is "debug" with --profile dev...
    let profile_directory = match profile {
        "dev" => "debug",
        other => other,
    };

    symlink(repository_root.join("etc"), upload_directory.join("etc"))
        .await
        .wrap_err("failed to link etc directory")?;

    create_dir_all(upload_directory.join("bin"))
        .await
        .wrap_err("failed to create directory for binaries")?;

    let hulk_binary = format!("target/x86_64-aldebaran-linux-gnu/{profile_directory}/hulk_nao");
    symlink(
        repository_root.join(hulk_binary),
        upload_directory.join("bin/hulk"),
    )
    .await
    .wrap_err("failed to link executable")?;

    Ok(())
}
