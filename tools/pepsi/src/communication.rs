use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use repository::Repository;

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum Arguments {
    Enable,
    Disable,
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> Result<()> {
    let enable = arguments == Arguments::Enable;
    repository
        .configure_communication(enable)
        .await
        .wrap_err("failed to set communication enablement")
}
