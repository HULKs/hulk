use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to power off e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn power_off(arguments: Arguments) -> Result<()> {
    let multi_progress = ProgressIndicator::new();

    let tasks = arguments.naos.into_iter().map(|nao_address| {
        let multi_progress = multi_progress.clone();
        async move {
            let progress = multi_progress.task(nao_address.to_string());
            progress.set_message("Powering off...");

            let nao = Nao::new(nao_address.ip);

            progress.finish_with(
                nao.power_off()
                    .await
                    .wrap_err_with(|| format!("failed to power {nao_address} off")),
            )
        }
    });

    join_all(tasks).await;

    Ok(())
}
