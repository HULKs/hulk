use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Context, ContextCompat, Result};
use pathdiff::diff_paths;

use crate::Repository;

impl Repository {
    pub fn root_to_current_dir(&self) -> Result<PathBuf> {
        let current_dir = current_dir().wrap_err("failed to get current directory")?;
        let path = diff_paths(&current_dir, &self.root)
            .wrap_err("failed to express current directory relative to repository root")?;
        Ok(Path::new("./").join(path))
    }

    pub fn current_dir_to_root(&self) -> Result<PathBuf> {
        let current_dir = current_dir().wrap_err("failed to get current directory")?;
        let path = diff_paths(&self.root, &current_dir)
            .wrap_err("failed to express repository root relative to current directory")?;
        Ok(Path::new("./").join(path))
    }
}
