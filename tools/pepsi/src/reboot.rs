use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to reboot e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn reboot(arguments: Arguments) -> Result<()> {
    let multi_progress = ProgressIndicator::new();

    let tasks = arguments.naos.into_iter().map(|nao_address| {
        let multi_progress = multi_progress.clone();
        async move {
            let progress = multi_progress.task(nao_address.to_string());
            progress.set_message("Rebooting...");

            let nao = Nao::new(nao_address.ip);

            progress.finish_with(
                nao.reboot()
                    .await
                    .wrap_err_with(|| format!("failed to reboot {nao_address}")),
            );
        }
    });

    join_all(tasks).await;

    Ok(())
}
