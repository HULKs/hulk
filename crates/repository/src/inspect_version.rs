use std::{fs::read_to_string, path::Path};

use color_eyre::{eyre::Context, Result};
use semver::Version;
use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize, Debug)]
struct Cargo {
    package: Package,
}

#[derive(Deserialize, Debug)]
struct Package {
    version: String,
}

/// Inspects and returns the version of a package from its `Cargo.toml` file.
pub fn inspect_version(toml_path: impl AsRef<Path>) -> Result<Version> {
    let toml_path = toml_path.as_ref();
    let cargo_toml_text = read_to_string(toml_path).wrap_err("failed to read file")?;
    let cargo_toml: Cargo = toml::from_str(&cargo_toml_text).wrap_err("failed to parse content")?;
    let raw_version = &cargo_toml.package.version;
    let version = Version::parse(raw_version)
        .wrap_err_with(|| format!("failed to parse version '{raw_version}' as SemVer"))?;
    Ok(version)
}

/// Checks whether the package has a newer version than the provided version.
pub fn check_for_update(own_version: &str, cargo_toml: impl AsRef<Path>) -> Result<()> {
    let own_version = Version::parse(own_version)
        .wrap_err_with(|| format!("failed to parse own version '{own_version}' as SemVer"))?;
    let cargo_toml_version = inspect_version(&cargo_toml).wrap_err_with(|| {
        format!(
            "failed to inspect version of package at {}",
            cargo_toml.as_ref().display()
        )
    })?;
    if own_version < cargo_toml_version {
        let crate_path = cargo_toml.as_ref().parent().unwrap();
        warn!(
            "New version available!
        Own version: {own_version}
        New version: {cargo_toml_version}
        To install new version use:
            cargo install --path {}",
            crate_path.display()
        );
    }
    Ok(())
}
