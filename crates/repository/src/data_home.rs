use std::{env, path::PathBuf};

use color_eyre::Result;

pub const HULK_DATA_HOME: &str = "HULK_DATA_HOME";

/// Get the data home directory.
///
/// This function returns the directory where hulk stores its data. The directory is determined by
/// the `HULK_DATA_HOME` environment variable. If the environment variable is not set, the
/// user-specific data directory (set by `XDG_DATA_HOME`) is used.
pub fn get_data_home() -> Result<PathBuf> {
    if let Ok(home) = env::var(HULK_DATA_HOME) {
        return Ok(PathBuf::from(home));
    }

    let base_directories = xdg::BaseDirectories::with_prefix("hulk")?;
    Ok(base_directories.get_data_home())
}
