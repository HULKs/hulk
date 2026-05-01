mod app;
pub mod cli;
mod commands;
mod model;
mod render;
mod support;

use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    cli::{Cli, Command},
    render::OutputMode,
};

pub async fn run(cli: Cli) -> Result<()> {
    let Cli {
        router,
        domain,
        json,
        command,
    } = cli;
    let output_mode = OutputMode::from_json_flag(json);

    match command {
        Command::Inspect(args) => commands::inspect::run(output_mode, &args),
        command => run_online_command(router, domain, output_mode, command).await,
    }
}

async fn run_online_command(
    router: String,
    domain: usize,
    output_mode: OutputMode,
    command: Command,
) -> Result<()> {
    let app = AppContext::new(&router, domain).await?;

    let result = match command {
        Command::List { target } => commands::list::run(&app, output_mode, target).await,
        Command::Watch => commands::watch::run(&app, output_mode).await,
        Command::Graph => commands::graph::run(&app, output_mode).await,
        Command::Schema {
            type_name,
            node,
            schema_hash,
        } => commands::schema::run(&app, output_mode, &node, &type_name, &schema_hash).await,
        Command::Parameter { command } => {
            commands::parameter::run(&app, output_mode, command).await
        }
        Command::Echo {
            topic,
            count,
            timeout,
        } => commands::echo::run(&app, output_mode, &topic, count, timeout).await,
        Command::Record(args) => {
            commands::record::run(&app, output_mode, &router, domain, &args).await
        }
        Command::Info { target, name } => {
            commands::info::run(&app, output_mode, target, &name).await
        }
        Command::Inspect(_) => unreachable!("inspect is handled before AppContext creation"),
    };
    let shutdown_result = app.shutdown();

    match (result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::{cli::Cli, run};

    #[tokio::test]
    async fn inspect_command_skips_app_context_creation() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let cli = Cli::parse_from([
            "rosz",
            "--router",
            "definitely-not-a-valid-router-endpoint",
            "inspect",
            file.path().to_str().expect("temp file path"),
        ]);

        let error = run(cli)
            .await
            .expect_err("empty file should not inspect successfully");
        assert!(!error.to_string().contains("failed to build ros-z context"));
    }
}
