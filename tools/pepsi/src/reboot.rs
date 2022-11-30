use anyhow::Context;
use clap::Args;
use futures::future::join_all;

use nao::Nao;
use repository::Repository;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to reboot e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn reboot(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let tasks = arguments.naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.private_key_path());

        nao.reboot()
            .await
            .with_context(|| format!("Failed to reboot {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some reboot tasks")?;

    Ok(())
}
