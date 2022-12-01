use std::path::PathBuf;

use anyhow::Context;
use clap::Subcommand;
use futures::future::join_all;

use nao::Nao;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(Subcommand)]
pub enum Arguments {
    // Delete logs on the NAOs
    Delete {
        /// The NAOs to delete logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
    // Download logs from the NAOs
    Download {
        /// Directory where to store the downloaded logs (will be created if not existing)
        log_directory: PathBuf,
        /// The NAOs to download logs from e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub async fn logs(arguments: Arguments) -> anyhow::Result<()> {
    let results = match arguments {
        Arguments::Delete { naos } => {
            join_all(naos.into_iter().map(|nao_address| async move {
                let nao = Nao::new(nao_address.ip);
                nao.delete_logs()
                    .await
                    .with_context(|| format!("Failed to delete logs on {nao_address}"))
            }))
            .await
        }
        Arguments::Download {
            log_directory,
            naos,
        } => {
            join_all(naos.into_iter().map(|nao_address| {
                let log_directory = log_directory.join(nao_address.to_string());
                async move {
                    let nao = Nao::new(nao_address.ip);
                    nao.download_logs(log_directory)
                        .await
                        .with_context(|| format!("Failed to download logs on {nao_address}"))
                }
            }))
            .await
        }
    };

    gather_results(results, "Failed to execute some delete/download logs tasks")?;

    Ok(())
}
