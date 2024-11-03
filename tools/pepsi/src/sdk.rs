use std::path::Path;

use clap::Subcommand;
use color_eyre::{eyre::Context, Result};

use repository::{
    configuration::get_sdk_version, data_home::get_data_home, sdk::download_and_install,
};

#[derive(Subcommand)]
pub enum Arguments {
    Install {
        /// SDK version e.g. `3.3.1`. If not provided, version specified by `hulk.toml` is be used.
        #[arg(long)]
        version: Option<String>,
    },
    Path,
}

pub async fn sdk(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    match arguments {
        Arguments::Install { version } => {
            let data_home = get_data_home()?;
            let version = match version {
                Some(version) => version,
                None => get_sdk_version(repository_root)
                    .await
                    .wrap_err("failed to get OS version")?,
            };
            download_and_install(&version, data_home).await?;
        }
        Arguments::Path => {
            let sdk_version = get_sdk_version(&repository_root)
                .await
                .wrap_err("failed to get HULK OS version")?;
            let data_home = get_data_home().wrap_err("failed to get data home")?;
            let path = &data_home.join(format!("sdk/{sdk_version}/"));
            println!("{}", path.display());
        }
    }

    Ok(())
}
