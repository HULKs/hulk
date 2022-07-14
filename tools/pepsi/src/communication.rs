use anyhow::Context;
use clap::Subcommand;
use futures::future::join_all;

use repository::Repository;

use crate::{parsers::NaoNumber, results::gather_results};

#[derive(Subcommand)]
pub enum Arguments {
    Enable {
        /// The NAO number to enable communication on e.g. 20 or 32
        #[clap(required = true)]
        nao_numbers: Vec<NaoNumber>,
    },
    Disable {
        /// The NAO number to disable communication on e.g. 20 or 32
        #[clap(required = true)]
        nao_numbers: Vec<NaoNumber>,
    },
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let hardware_ids = repository
        .get_hardware_ids()
        .await
        .context("Failed to get hardware IDs")?;

    let (enable, nao_numbers) = match arguments {
        Arguments::Enable { nao_numbers } => (true, nao_numbers),
        Arguments::Disable { nao_numbers } => (false, nao_numbers),
    };

    let tasks = nao_numbers.into_iter().map(|nao_number| {
        let head_id = &hardware_ids[&nao_number.number].head_id;
        async move {
            repository
                .set_communication(head_id, enable)
                .await
                .with_context(|| format!("Failed to set communication enablement for {nao_number}"))
        }
    });

    let results = join_all(tasks).await;
    gather_results(
        results,
        "Failed to execute some communication setting tasks",
    )?;

    Ok(())
}
