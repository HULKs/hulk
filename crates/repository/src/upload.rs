use std::path::Path;

use color_eyre::{eyre::Context, Result};
use tokio::fs::{create_dir_all, symlink};

use crate::Repository;

impl Repository {
    pub async fn populate_upload_directory(
        &self,
        upload_directory: impl AsRef<Path>,
        hulk_binary: impl AsRef<Path>,
    ) -> Result<()> {
        let upload_directory = upload_directory.as_ref();

        symlink(self.root.join("etc"), upload_directory.join("etc"))
            .await
            .wrap_err("failed to link etc directory")?;

        create_dir_all(upload_directory.join("bin"))
            .await
            .wrap_err("failed to create directory for binaries")?;
        symlink(
            self.root.join(hulk_binary),
            upload_directory.join("bin/hulk"),
        )
        .await
        .wrap_err("failed to link executable")?;

        Ok(())
    }
}

pub fn get_hulk_binary(profile: &str) -> String {
    // the target directory is "debug" with --profile dev...
    let profile_directory = match profile {
        "dev" => "debug",
        other => other,
    };

    format!("target/x86_64-aldebaran-linux-gnu/{profile_directory}/hulk_nao")
}
