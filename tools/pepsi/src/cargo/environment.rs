use std::str::FromStr;

use clap::Args;
use color_eyre::eyre::{bail, Context, Error, Result};
use repository::{cargo::Environment as RepositoryEnvironment, sdk::SDKImage, Repository};

#[derive(Args, Debug, Clone)]
pub struct EnvironmentArguments {
    /// The execution environment (default: native)
    #[arg(long)]
    pub env: Option<Environment>,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Podman { image: Option<String> },
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
            "podman" => Self::Podman {
                image: right.map(str::to_owned),
            },
            "docker" => Self::Docker {
                image: right.map(str::to_owned),
            },
            _ => bail!("unknown option {left}"),
        })
    }
}

impl Environment {
    pub async fn resolve(self, repository: &Repository) -> Result<RepositoryEnvironment> {
        let sdk_version = repository
            .read_sdk_version()
            .await
            .wrap_err("failed to get HULK OS version")?;

        let sdk_image = SDKImage {
            registry: "ghcr.io/hulks".to_string(),
            name: "k1sdk".to_string(),
            tag: sdk_version,
        };

        Ok(match self {
            Environment::Native => RepositoryEnvironment::Native,
            Environment::Podman { image: Some(image) } => RepositoryEnvironment::Podman {
                sdk_image: { sdk_image.parse_and_update(&image) },
            },
            Environment::Podman { image: None } => RepositoryEnvironment::Podman { sdk_image },
            Environment::Docker { image: Some(image) } => RepositoryEnvironment::Docker {
                sdk_image: { sdk_image.parse_and_update(&image) },
            },
            Environment::Docker { image: None } => RepositoryEnvironment::Docker { sdk_image },
        })
    }
}
