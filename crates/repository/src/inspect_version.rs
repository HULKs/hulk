use std::{fs::read_to_string, path::Path};

use color_eyre::{eyre::Context, Result};
use log::warn;
use semver::Version;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Cargo {
    package: Package,
}
#[derive(Deserialize, Debug)]
struct Package {
    version: String,
}

/// Inspects and returns the version of a Cargo package from its `Cargo.toml` file.
///
/// This function takes a path to a `Cargo.toml` file, reads the file,
/// parses the contents to extract the package version, and returns the version.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The `Cargo.toml` file cannot be read.
/// * The contents of the `Cargo.toml` file cannot be parsed.
/// * The version string extracted from the `Cargo.toml` file cannot be parsed into a `Version` object.
///
/// # Example
///
/// ```no_run
/// let version = inspect_version(Path::new("path/to/Cargo.toml")).expect("failed to inspect version");
/// println!("Package version: {}", version);
/// ```
pub fn inspect_version(path: impl AsRef<Path>) -> Result<Version> {
    let path = path.as_ref();
    let cargo_toml_text = read_to_string(path)
        .wrap_err_with(|| format!("failed to load Cargo.toml at {}", path.display()))?;
    let cargo_toml: Cargo = toml::from_str(&cargo_toml_text)
        .wrap_err_with(|| format!("failed to parse package version from {}", path.display()))?;
    let version = Version::parse(&cargo_toml.package.version).wrap_err_with(|| {
        format!(
            "failed to parse package version '{}' from {}",
            cargo_toml.package.version,
            path.display()
        )
    })?;
    Ok(version)
}

/// Checks if there is a new version of the crate available.
///
/// This function takes the current version of a package and the path to a `Cargo.toml` file,
/// reads and parses the version from the `Cargo.toml` file, and compares it with the current version.
/// If the version in the `Cargo.toml` file is newer, it prints a message indicating that a new version
/// is available and provides the command to install the new version.
///
/// # Errors
///
/// This function will return an error if:
///
/// * The `own_version` string cannot be parsed into a `Version` object.
/// * The `Cargo.toml` file cannot be read or parsed to extract the package version.
///
/// # Example
///
/// ```no_run
/// let current_version = "1.0.0";
/// let cargo_toml_path = Path::new("path/to/Cargo.toml");
///
/// check_for_update(current_version, &cargo_toml_path).expect("failed to check for update");
/// ```
pub fn check_for_update(own_version: &str, cargo_toml: impl AsRef<Path>) -> Result<()> {
    let own_version = Version::parse(own_version)?;
    let cargo_toml_version = inspect_version(&cargo_toml)?;
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
