use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::Result;

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
            repository
                .link_and_install_sdk(sdk_version.as_deref(), installation_directory.as_deref())
                .await
        }
    }
}
