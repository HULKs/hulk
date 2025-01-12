use std::{ffi::OsString, path::Path};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::{
    fs::{create_dir_all, symlink},
    process::Command,
};

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

        create_dir_all(upload_directory.join("logs"))
            .await
            .wrap_err("failed to create directory for logs")?;

        let source_file = upload_directory.join("logs/source.tar.gz");

        let mut archive_command = OsString::from("cd ");
        archive_command.push(&self.root);
        archive_command.push(" && git ls-files --cached --others --exclude-standard | tar Tczf - ");
        archive_command.push(source_file);

        let mut command = Command::new("sh");
        command.arg("-c").arg(archive_command);

        let status = command
            .status()
            .await
            .wrap_err("failed to run archive command")?;

        if !status.success() {
            bail!("archive command failed with {status}")
        }

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
