use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use nao::Nao;

use crate::{parsers::NaoAddress, progress_indicator::ProgressIndicator};

#[derive(Args)]
pub struct Arguments {
    /// The NAOs to reboot e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub naos: Vec<NaoAddress>,
}

pub async fn reboot(arguments: Arguments) -> Result<()> {
    ProgressIndicator::map_tasks(arguments.naos, "Rebooting...", |nao_address| async move {
        let nao = Nao::try_new_with_ping(nao_address.ip).await?;
        nao.reboot()
            .await
            .wrap_err_with(|| format!("failed to reboot {nao_address}"))
    })
    .await;

    Ok(())
}
