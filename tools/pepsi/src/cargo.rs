use anyhow::Context;
use repository::Repository;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Arguments {
    #[structopt(long, default_value = "incremental")]
    pub profile: String,
    #[structopt(long, default_value = "webots")]
    pub target: String,
    #[structopt(long)]
    pub no_sdk_installation: bool,
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
            .build(arguments.profile, arguments.target)
            .await
            .context("Failed to build")?,
        Command::Check => repository
            .check(arguments.profile, arguments.target)
            .await
            .context("Failed to check")?,
        Command::Clippy => repository
            .clippy(arguments.profile, arguments.target)
            .await
            .context("Failed to run clippy")?,
        Command::Run => repository
            .run(arguments.profile, arguments.target)
            .await
            .context("Failed to run")?,
    }

    Ok(())
}
