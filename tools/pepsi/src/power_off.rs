use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to power off e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn power_off(arguments: Arguments) -> Result<()> {
    ProgressIndicator::map_tasks(
        arguments.naos,
        "Powering off...",
        |nao_address| async move {
            let nao = Nao::new_with_ping(nao_address.ip).await?;
            nao.power_off()
                .await
                .wrap_err_with(|| format!("failed to power {nao_address} off"))
        },
    )
    .await;

    Ok(())
}
