use anyhow::Context;
use futures::future::join_all;
use nao::Nao;
use repository::Repository;
use structopt::StructOpt;

use crate::{parsers::NaoAddress, results::gather_results};

#[derive(StructOpt)]
pub enum Arguments {
    Enable {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
    Disable {
        /// The NAOs to execute that command on e.g. 20w or 10.1.24.22
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub async fn aliveness(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let (enable, naos) = match arguments {
        Arguments::Enable { naos } => (true, naos),
        Arguments::Disable { naos } => (false, naos),
    };

    let tasks = naos.into_iter().map(|nao_address| async move {
        let nao = Nao::new(nao_address.to_string(), repository.get_private_key_path());

        nao.set_aliveness(enable)
            .await
            .with_context(|| format!("Failed to set aliveness on {nao_address}"))
    });

    let results = join_all(tasks).await;
    gather_results(results, "Failed to execute some aliveness setting tasks")?;

    Ok(())
}
