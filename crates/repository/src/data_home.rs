use std::path::PathBuf;

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::process::Command;

use crate::Repository;

impl Repository {
    /// Get the data home directory.
    ///
    /// This function returns the directory where hulk stores its data. The directory is determined by
    /// the `HULK_DATA_HOME` environment variable. If the environment variable is not set, the
    /// user-specific data directory (set by `XDG_DATA_HOME`) is used.
    pub async fn resolve_data_home(&self) -> Result<PathBuf> {
        let output = Command::new(self.root.join("scripts/resolve_data_home"))
            .output()
            .await
            .wrap_err("failed to spawn resolve_data_home script")?;

        if !output.status.success() {
            bail!("failed to resolve data home");
        }

        let data_home = String::from_utf8(output.stdout).wrap_err("failed to parse data home")?;
        Ok(PathBuf::from(data_home))
    }

    pub fn data_home_script(&self) -> Result<String> {
        let root = self.current_dir_to_root()?;
        Ok(format!(
            "{root}/scripts/resolve_data_home",
            root = root.display(),
        ))
    }
}
