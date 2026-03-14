use clap::Subcommand;
use color_eyre::{Result, eyre::WrapErr};

use parameters::directory::LocationTarget;
use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    /// List all available locations
    #[command(visible_alias = "aufzähl")]
    List,
    /// Set location for repository
    #[command(visible_alias = "setz")]
    Set {
        /// The target to set a location for
        #[arg(required = true)]
        target: LocationTarget,
        /// The location to set for the repository
        #[arg(required = true)]
        location: String,
    },
    /// Get currently configured locations
    #[command(visible_alias = "zustand")]
    Status,
}

pub async fn location(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::List => {
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
                .set_location(target, &location)
                .await
                .wrap_err_with(|| format!("failed setting location for {target}"))?;
        }
        Arguments::Status => {
            println!("Configured Locations:");
            for (target, location) in repository
                .list_configured_locations()
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
