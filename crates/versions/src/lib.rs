use std::fs::read_to_string;
use std::path::Path;

use color_eyre::{eyre::Context, Result};
use semver::Version;
use serde::Deserialize;
use toml::from_str;

pub fn check_version(path: impl AsRef<Path>) -> Result<bool> {
    #[derive(Deserialize, Debug)]
    struct Cargo {
        package: Package,
    }
    #[derive(Deserialize, Debug)]
    struct Package {
        version: String,
    }

    let own_version =
        Version::parse(env!("CARGO_PKG_VERSION")).wrap_err("failed to parse own version")?;
    let cargo_toml_path = path.as_ref().join("Cargo.toml");
    let cargo_toml_text = read_to_string(&cargo_toml_path).wrap_err_with(|| {
        format!(
            "failed to load cargo toml at {}",
            cargo_toml_path.to_str().unwrap()
        )
    })?;
    let cargo_toml: Cargo = from_str(&cargo_toml_text).wrap_err_with(|| {
        format!(
            "failed to parse package version from {}",
            cargo_toml_path.to_str().unwrap()
        )
    })?;
    let cargo_toml_version = Version::parse(&cargo_toml.package.version).unwrap();
    if own_version < cargo_toml_version {
        println!("outdated version!");
        println!("You are using     {own_version}");
        println!("But repo contains {cargo_toml_version}");
    }

    Ok(own_version >= cargo_toml_version)
}
