use clap::{Args, Subcommand};
use color_eyre::{Result, eyre::Context};

use repository::{
    Repository,
    sdk::{
        ContainerRuntime, SDKImage, build_sdk_container_with, list_sdk_images_with,
        pull_sdk_image_with,
    },
};

#[derive(Args, Debug, Clone, Copy)]
pub struct RuntimeArguments {
    /// Container runtime to use. Defaults to Docker on macOS and Podman elsewhere.
    #[arg(long, value_name = "RUNTIME")]
    runtime: Option<ContainerRuntime>,
}

impl RuntimeArguments {
    fn resolve(self) -> ContainerRuntime {
        self.runtime
            .unwrap_or_else(ContainerRuntime::default_for_host)
    }
}

#[derive(Subcommand)]
pub enum Arguments {
    /// Pulls the SDK image from ghcr.io/hulks
    #[command(visible_alias = "installier")]
    Install {
        /// SDK version e.g. `1.0.0`. If not provided, version specified by `hulk.toml` is used.
        #[arg(long, visible_alias = "abbild", visible_alias = "bild")]
        image: Option<String>,
        #[command(flatten)]
        runtime: RuntimeArguments,
    },
    /// Builds the SDK image
    #[command(visible_alias = "bau")]
    Build {
        /// SDK version e.g. `3.3.1`. If not provided, version specified by `hulk.toml` is used.
        #[arg(long, visible_alias = "abbild", visible_alias = "bild")]
        image: Option<String>,
        #[command(flatten)]
        runtime: RuntimeArguments,
    },
    #[command(visible_alias = "aufzähl")]
    List {
        #[command(flatten)]
        runtime: RuntimeArguments,
    },
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
        Arguments::Install { image, runtime } => {
            if let Some(image) = image {
                sdk_image = sdk_image.parse_and_update(&image)
            }

            pull_sdk_image_with(&sdk_image, runtime.resolve()).await?;
        }
        Arguments::Build { image, runtime } => {
            if let Some(image) = image {
                sdk_image = sdk_image.parse_and_update(&image)
            }

            build_sdk_container_with(repository, &sdk_image, runtime.resolve()).await?;
        }
        Arguments::List { runtime } => {
            list_sdk_images_with(runtime.resolve()).await?;
        }
    }

    Ok(())
}
