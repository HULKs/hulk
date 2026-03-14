use clap::Subcommand;
use color_eyre::{Result, eyre::WrapErr};
use repository::Repository;

#[derive(Subcommand, PartialEq, Eq, Debug)]
pub enum Arguments {
    #[command(visible_alias = "an")]
    Enable,
    #[command(visible_alias = "aus")]
    Disable,
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> Result<()> {
    let enable = arguments == Arguments::Enable;
    repository
        .configure_communication(enable)
        .await
        .wrap_err("failed to set communication enablement")
}
