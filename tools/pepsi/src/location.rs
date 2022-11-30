use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

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

pub async fn location(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::List {} => {
            println!("Available Locations:");
            for location in repository
                .list_available_locations()
                .await
                .wrap_err("failed to list available locations")?
            {
                println!("- {location}");
            }
        }
        Arguments::Set { target, location } => {
            repository
                .set_location(&target, &location)
                .await
                .wrap_err_with(|| format!("failed setting location for {target}"))?;
        }
        Arguments::Status {} => {
            println!("Configured Locations:");
            for (target, location) in repository
                .get_configured_locations()
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
