use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use futures_util::{stream::FuturesUnordered, StreamExt};

use repository::Repository;

use crate::{parsers::NaoNumber, progress_indicator::ProgressIndicator};

#[derive(Subcommand)]
pub enum Arguments {
    Enable {
        /// The NAO number to enable communication on e.g. 20 or 32
        #[arg(required = true)]
        nao_numbers: Vec<NaoNumber>,
    },
    Disable {
        /// The NAO number to disable communication on e.g. 20 or 32
        #[arg(required = true)]
        nao_numbers: Vec<NaoNumber>,
    },
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> Result<()> {
    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .wrap_err("failed to get hardware IDs")?;

    let (enable, nao_numbers) = match arguments {
        Arguments::Enable { nao_numbers } => (true, nao_numbers),
        Arguments::Disable { nao_numbers } => (false, nao_numbers),
    };

    let multi_progress = ProgressIndicator::new();

    nao_numbers
        .into_iter()
        .map(|nao_number| {
            let multi_progress = multi_progress.clone();
            let head_id = &hardware_ids[&nao_number.number].head_id;
            async move {
                let progress = multi_progress.task(format!("{}", nao_number));

                progress.set_message("Setting communication...");

                progress.finish_with(
                    repository
                        .set_communication(head_id, enable)
                        .await
                        .wrap_err_with(|| {
                            format!("failed to set communication enablement for {nao_number}")
                        }),
                )
            }
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
