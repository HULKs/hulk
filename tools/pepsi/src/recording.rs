use std::collections::HashSet;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;

#[derive(Args)]
pub struct Arguments {
    /// Cycler instances to record e.g. Control or VisionBottom (call without cycler instances to disable recording)
    pub cycler_instances_to_be_recorded: Vec<String>,
}

pub async fn recording(arguments: Arguments, repository: &Repository) -> Result<()> {
    repository
        .set_cycler_instances_to_be_recorded(HashSet::from_iter(
            arguments.cycler_instances_to_be_recorded,
        ))
        .await
        .wrap_err("failed to set recording enablement")
}
