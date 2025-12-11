use clap::Subcommand;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};

use repository::{
    sdk::{build_sdk_container, pull_sdk_image, SDKImage},
    Repository,
};
use tokio::process::Command;

#[derive(Subcommand)]
pub enum Arguments {
    /// Pulls the SDK image from ghcr.io/hulks
    Install {
        /// SDK version e.g. `1.0.0`. If not provided, version specified by `hulk.toml` is used.
        #[arg(long)]
        image: Option<String>,
    },
    /// Builds the SDK image
    Build {
        /// SDK version e.g. `3.3.1`. If not provided, version specified by `hulk.toml` is used.
        #[arg(long)]
        image: Option<String>,
    },
    List,
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    let sdk_version = repository
        .read_sdk_version()
        .await
        .wrap_err("failed to get HULK OS version")?;

    let mut sdk_image = SDKImage {
        registry: "ghcr.io/hulks".to_string(),
        name: "k1sdk".to_string(),
        tag: sdk_version,
    };

    match arguments {
        Arguments::Install { image } => {
            if let Some(image) = image {
                sdk_image = sdk_image.parse_and_update(&image)
            }

            pull_sdk_image(&sdk_image).await?;
        }
        Arguments::Build { image } => {
            if let Some(image) = image {
                sdk_image = sdk_image.parse_and_update(&image)
            }

            build_sdk_container(repository, &sdk_image).await?;
        }
        Arguments::List => {
            let status = Command::new("podman")
                .args(["image", "list", "--filter", "reference=k1sdk"])
                .status()
                .await?;

            if !status.success() {
                bail!("podman failed with {status}");
            }
        }
    }

    Ok(())
}
