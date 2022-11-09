use anyhow::Context;
use clap::Args;
use futures::future::join_all;

use nao::Nao;
use repository::Repository;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to power off e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn power_off(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let tasks = arguments.naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        nao.power_off()
            .await
            .with_context(|| format!("Failed to power {nao_address} off"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some power_off tasks")?;

    Ok(())
}
