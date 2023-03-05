use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use futures_util::{stream::FuturesUnordered, StreamExt};

use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

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

pub async fn logs(arguments: Arguments) -> Result<()> {
    let multi_progress = ProgressIndicator::new();

    match arguments {
        Arguments::Delete { naos } => {
            naos.into_iter()
                .map(|nao_address| {
                    let multi_progress = multi_progress.clone();
                    async move {
                        let progress = multi_progress.task(nao_address.to_string());
                        progress.set_message("Deleting logs...");

                        let nao = Nao::new(nao_address.ip);

                        progress.finish_with(
                            nao.delete_logs().await.wrap_err_with(|| {
                                format!("failed to delete logs on {nao_address}")
                            }),
                        )
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await
        }
        Arguments::Download {
            log_directory,
            naos,
        } => {
            naos.into_iter()
                .map(|nao_address| {
                    let multi_progress = multi_progress.clone();
                    let log_directory = log_directory.join(nao_address.to_string());
                    async move {
                        let progress = multi_progress.task(nao_address.to_string());
                        progress.set_message("Downloading logs...");

                        let nao = Nao::new(nao_address.ip);

                        progress.finish_with(
                            nao.download_logs(log_directory).await.wrap_err_with(|| {
                                format!("failed to download logs on {nao_address}")
                            }),
                        )
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await
        }
    };

    Ok(())
}
