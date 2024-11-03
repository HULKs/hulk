use std::path::Path;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use repository::communication::configure_communication;

#[derive(Subcommand)]
pub enum Arguments {
    Enable,
    Disable,
}

pub async fn communication(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    configure_communication(matches!(arguments, Arguments::Enable), repository_root)
        .await
        .wrap_err("failed to set communication enablement")
}
