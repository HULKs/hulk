mod app;
pub mod cli;
mod commands;
mod model;
mod render;
mod support;

use color_eyre::eyre::Result;

use crate::{app::AppContext, cli::Command, render::OutputMode};

pub async fn run(cli: crate::cli::Cli) -> Result<()> {
    let crate::cli::Cli {
        router,
        json,
        command,
    } = cli;
    let output_mode = OutputMode::from_json_flag(json);

    run_online_command(router, output_mode, command).await
}

async fn run_online_command(
    router: String,
    output_mode: OutputMode,
    command: Command,
) -> Result<()> {
    let app = AppContext::new(&router).await?;

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
        Command::Info { target, name } => {
            commands::info::run(&app, output_mode, target, &name).await
        }
    };
    let shutdown_result = app.shutdown();

    match (result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}
