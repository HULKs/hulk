use anyhow::{anyhow, Context};
use clap::Subcommand;

use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    /// List all available locations
    List,
    /// Set location for repository
    Set {
        /// The target to set a location for (nao, webots, behavior_simulator)
        #[arg(required = true)]
        target: String,
        /// The location to set for the repository
        #[arg(required = true)]
        location: String,
    },
    /// Get currently configured locations
    Status,
}

pub async fn location(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    match arguments {
        Arguments::List {} => {
            println!("Available Locations:");
            for location in repository
                .list_available_locations()
                .await
                .context("Failed to list available locations")?
            {
                println!("- {location}");
            }
        }
        Arguments::Set { target, location } => {
            repository
                .set_location(&target, &location)
                .await
                .with_context(|| anyhow!("Failed setting location for {target}"))?;
        }
        Arguments::Status {} => {
            println!("Configured Locations:");
            for (target, location) in repository
                .get_configured_locations()
                .await
                .context("Failed to get configured locations")?
            {
                println!(
                    "- {target:30}{}",
                    location.unwrap_or_else(|| "<NOT_CONFIGURED>".to_string())
                );
            }
        }
    };
    Ok(())
}
