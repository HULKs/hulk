use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;

use crate::progress_indicator::ProgressIndicator;

#[derive(Subcommand)]
pub enum Arguments {
    Enable,
    Disable,
}

pub async fn communication(arguments: Arguments, repository: &Repository) -> Result<()> {
    let multi_progress = ProgressIndicator::new();
    multi_progress
        .task("Setting communication...".to_string())
        .finish_with(
            repository
                .set_communication(matches!(arguments, Arguments::Enable))
                .wrap_err("failed to set communication enablement"),
        );

    Ok(())
}
