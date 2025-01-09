use std::{path::Path, str::FromStr};

use clap::Args;
use color_eyre::eyre::{bail, Context, Error, Result};
use repository::{cargo::Environment as RepositoryEnvironment, configuration::read_sdk_version};

#[derive(Args, Debug)]
pub struct EnvironmentArguments {
    /// Use an SDK execution environment (default: native)
    #[arg(long, require_equals = true, default_value = "native")]
    pub env: Environment,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Sdk { version: Option<String> },
    Docker { image: Option<String> },
}

impl FromStr for Environment {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (left, right) = string
            .split_once(":")
            .map_or((string, None), |(left, right)| (left, Some(right)));

        Ok(match left {
            "native" => Self::Native,
            "sdk" => Self::Sdk {
                version: right.map(str::to_owned),
            },
            "docker" => Self::Docker {
                image: right.map(str::to_owned),
            },
            _ => bail!("unknown option {left}"),
        })
    }
}

impl Environment {
    pub async fn resolve(self, repository_root: impl AsRef<Path>) -> Result<RepositoryEnvironment> {
        let sdk_version = read_sdk_version(&repository_root)
            .await
            .wrap_err("failed to get HULK OS version")?;

        Ok(match self {
            Environment::Native => RepositoryEnvironment::Native,
            Environment::Sdk { version } => RepositoryEnvironment::Sdk {
                version: version.unwrap_or(sdk_version),
            },
            Environment::Docker { image } => RepositoryEnvironment::Docker {
                image: image.unwrap_or(format!("ghcr.io/hulks/naosdk:{sdk_version}")),
            },
        })
    }
}
