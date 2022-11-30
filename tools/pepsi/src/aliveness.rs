use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use futures::future::join_all;

use nao::Nao;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(Subcommand)]
pub enum Arguments {
    Enable {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
    Disable {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[arg(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub async fn aliveness(arguments: Arguments) -> Result<()> {
    let (enable, naos) = match arguments {
        Arguments::Enable { naos } => (true, naos),
        Arguments::Disable { naos } => (false, naos),
    };

    let tasks = naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.ip);
        nao.set_aliveness(enable)
            .await
            .wrap_err_with(|| format!("failed to set aliveness on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "failed to execute some aliveness setting tasks")?;

    Ok(())
}
