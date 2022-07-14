use anyhow::Context;
use clap::Args;

use repository::Repository;

#[derive(Args)]
pub struct Arguments {
    /// Apply to entire workspace (only valid for build/check/clippy)
    #[clap(long)]
    pub workspace: bool,
    #[clap(long, default_value = "incremental")]
    pub profile: String,
    #[clap(long, default_value = "webots")]
    pub target: String,
    #[clap(long)]
    pub no_sdk_installation: bool,
    /// Pass through arguments to cargo ... -- PASSTHROUGH_ARGUMENTS
    #[clap(last = true, value_parser)]
    pub passthrough_arguments: Vec<String>,
}

pub enum Command {
    Build,
    Check,
    Clippy,
    Run,
}

pub async fn cargo(
    arguments: Arguments,
    repository: &Repository,
    command: Command,
) -> anyhow::Result<()> {
    if !arguments.no_sdk_installation && arguments.target == "nao" {
        repository
            .install_sdk(false, None, None)
            .await
            .context("Failed to install SDK")?;
    }

    match command {
        Command::Build => repository
            .build(
                arguments.workspace,
                arguments.profile,
                arguments.target,
                arguments.passthrough_arguments,
            )
            .await
            .context("Failed to build")?,
        Command::Check => repository
            .check(arguments.workspace, arguments.profile, arguments.target)
            .await
            .context("Failed to check")?,
        Command::Clippy => repository
            .clippy(arguments.workspace, arguments.profile, arguments.target)
            .await
            .context("Failed to run clippy")?,
        Command::Run => {
            if arguments.workspace {
                println!("INFO: Found --workspace with run subcommand, ignoring...")
            }
            repository
                .run(
                    arguments.profile,
                    arguments.target,
                    arguments.passthrough_arguments,
                )
                .await
                .context("Failed to run")?
        }
    }

    Ok(())
}
