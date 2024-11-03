use std::path::Path;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};
use repository::location::{get_configured_locations, list_available_locations, set_location};

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

pub async fn location(arguments: Arguments, repository_root: impl AsRef<Path>) -> Result<()> {
    match arguments {
        Arguments::List {} => {
            println!("Available Locations:");
            for location in list_available_locations(repository_root)
                .await
                .wrap_err("failed to list available locations")?
            {
                println!("- {location}");
            }
        }
        Arguments::Set { target, location } => {
            set_location(&target, &location, repository_root)
                .await
                .wrap_err_with(|| format!("failed setting location for {target}"))?;
        }
        Arguments::Status {} => {
            println!("Configured Locations:");
            for (target, location) in get_configured_locations(repository_root)
                .await
                .wrap_err("failed to get configured locations")?
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
