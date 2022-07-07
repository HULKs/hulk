use std::path::PathBuf;

use anyhow::Context;
use repository::Repository;
use structopt::StructOpt;

#[derive(StructOpt)]
pub enum Arguments {
    Install {
        /// Force reinstallation of existing SDK
        #[structopt(long)]
        force_reinstall: bool,
        /// Alternative SDK version e.g. 3.3
        #[structopt(long)]
        sdk_version: Option<String>,
        /// Alternative SDK installation directory e.g. /opt/nao
        #[structopt(long)]
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
