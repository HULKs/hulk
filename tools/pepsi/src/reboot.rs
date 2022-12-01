use anyhow::Context;
use clap::Args;
use futures::future::join_all;

use nao::Nao;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to reboot e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn reboot(arguments: Arguments) -> anyhow::Result<()> {
    let tasks = arguments.naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        nao.reboot()
            .await
            .with_context(|| format!("Failed to reboot {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some reboot tasks")?;

    Ok(())
}
