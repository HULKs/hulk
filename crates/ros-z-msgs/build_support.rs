use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, bail};

pub fn discover_vendored_packages(names: &[&str], asset_root: &Path) -> Result<Vec<PathBuf>> {
    let mut packages = Vec::with_capacity(names.len());

    for name in names {
        let package_dir = asset_root.join(name);
        if !package_dir.exists() {
            bail!("vendored ROS interface package missing: {}", name);
        }

        packages.push(package_dir);
    }

    Ok(packages)
}
