use anyhow::Context;
use clap::Args;

use nao::Nao;
use repository::Repository;

use crate::parsers::NaoAddress;

#[derive(Args)]
pub struct Arguments {
    /// The NAO to connect to e.g. 20w or 10.1.24.22
    #[arg(required = true)]
    pub nao: NaoAddress,
}

pub async fn shell(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    let nao = Nao::new(arguments.nao.to_string(), repository.private_key_path());

    nao.execute_shell()
        .await
        .with_context(|| format!("Failed to execute shell on {}", arguments.nao))
}
