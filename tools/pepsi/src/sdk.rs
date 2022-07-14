use std::path::PathBuf;

use anyhow::Context;
use clap::Subcommand;

use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    Install {
        /// Force reinstallation of existing SDK
        #[clap(long)]
        force_reinstall: bool,
        /// Alternative SDK version e.g. 3.3
        #[clap(long)]
        sdk_version: Option<String>,
        /// Alternative SDK installation directory e.g. /opt/nao
        #[clap(long)]
        installation_directory: Option<PathBuf>,
    },
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    match arguments {
        Arguments::Install {
            force_reinstall,
            sdk_version,
            installation_directory,
        } => repository
            .install_sdk(force_reinstall, sdk_version, installation_directory)
            .await
            .context("Failed to install SDK")?,
    }

    Ok(())
}
