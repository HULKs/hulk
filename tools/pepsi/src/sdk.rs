use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use constants::OS_IS_NOT_LINUX;
use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    Install {
        /// Alternative SDK version e.g. 3.3
        #[arg(long)]
        sdk_version: Option<String>,
        /// Alternative SDK installation directory e.g. ~/.naosdk/
        #[arg(long)]
        installation_directory: Option<PathBuf>,
    },
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::Install {
            sdk_version,
            installation_directory,
        } => {
            let installation_directory = repository
                .link_sdk_home(installation_directory.as_deref())
                .await
                .wrap_err("failed to link SDK home")?;

            let use_docker = OS_IS_NOT_LINUX;
            if !use_docker {
                repository
                    .install_sdk(sdk_version.as_deref(), installation_directory)
                    .await
                    .wrap_err("failed to install SDK")?;
            }
        }
    }

    Ok(())
}
