use std::collections::HashSet;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;

#[derive(Args)]
pub struct Arguments {
    /// Cycler instances to record e.g. Control or VisionBottom (call without cycler instances to disable recording)
    pub recording_settings: Vec<String>,
}

pub async fn recording(arguments: Arguments, repository: &Repository) -> Result<()> {
    repository
        .set_recording_settings(HashSet::from_iter(arguments.recording_settings))
        .await
        .wrap_err("failed to set recording enablement")
}
