use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    Enable,
    Disable,
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> Result<()> {
    repository
        .set_communication(matches!(arguments, Arguments::Enable))
        .await
        .wrap_err("failed to set communication enablement")
}
