use clap::Subcommand;
use color_eyre::{eyre::Context, Result};

use repository::{sdk::download_and_install, Repository};

#[derive(Subcommand)]
pub enum Arguments {
    Install {
        /// SDK version e.g. `3.3.1`. If not provided, version specified by `hulk.toml` is used.
        #[arg(long)]
        version: Option<String>,
    },
    Path,
}

pub async fn sdk(arguments: Arguments, repository: &Repository) -> Result<()> {
    let data_home = repository
        .resolve_data_home()
        .await
        .wrap_err("failed to get data home")?;
    match arguments {
        Arguments::Install { version } => {
            let version = match version {
                Some(version) => version,
                None => repository
                    .read_sdk_version()
                    .await
                    .wrap_err("failed to get OS version")?,
            };
            download_and_install(&version, data_home).await?;
        }
        Arguments::Path => {
            let sdk_version = repository
                .read_sdk_version()
                .await
                .wrap_err("failed to get HULK OS version")?;
            let path = &data_home.join(format!("sdk/{sdk_version}/"));
            println!("{}", path.display());
        }
    }

    Ok(())
}
