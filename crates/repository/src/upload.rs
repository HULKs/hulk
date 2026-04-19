use std::path::Path;

use color_eyre::{
    Result,
    eyre::{Context, ContextCompat},
};
use tokio::fs::{create_dir_all, symlink};

use crate::Repository;

impl Repository {
    pub async fn populate_upload_directory(
        &self,
        upload_directory: impl AsRef<Path>,
        binaries: &[impl AsRef<Path>],
    ) -> Result<()> {
        let upload_directory = upload_directory.as_ref();

        symlink(self.root.join("etc"), upload_directory.join("etc"))
            .await
            .wrap_err("failed to link etc directory")?;

        create_dir_all(upload_directory.join("bin"))
            .await
            .wrap_err("failed to create directory for binaries")?;
        for binary in binaries {
            let binary = binary.as_ref();
            symlink(
                self.root.join(binary),
                upload_directory
                    .join("bin")
                    .join(binary.file_name().wrap_err_with(|| {
                        format!("could not determine filename of {}", binary.display())
                    })?),
            )
            .await
            .wrap_err_with(|| format!("failed to symlink executable {}", binary.display()))?;
        }

        create_dir_all(upload_directory.join("logs"))
            .await
            .wrap_err("failed to create directory for logs")?;

        symlink(&self.root, upload_directory.join("logs/source"))
            .await
            .wrap_err("failed to link source directory")?;

        Ok(())
    }
}

pub fn get_hulk_binary(profile: &str) -> String {
    // the target directory is "debug" with --profile dev...
    let profile_directory = match profile {
        "dev" => "debug",
        other => other,
    };

    format!("target/aarch64-unknown-linux-gnu/{profile_directory}/hulk_booster")
}
