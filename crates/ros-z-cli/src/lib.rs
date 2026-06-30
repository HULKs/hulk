//! Scriptable command-line companion for `ros-z` graphs, schemas, topics, and
//! parameters.

mod app;
pub mod cli;
mod commands;
mod model;
mod render;
mod support;

use clap::CommandFactory;
use color_eyre::eyre::Result;

use crate::{
    app::AppContext,
    cli::{Cli, Command, OnlineCommand},
    render::OutputMode,
};

/// Run the CLI with parsed command-line arguments.
pub async fn run(cli: Cli) -> Result<()> {
    let Cli {
        router,
        json,
        command,
    } = cli;

    match command {
        Command::Completions { shell } => {
            let mut command = Cli::command();
            clap_complete::generate(shell, &mut command, "rosz", &mut std::io::stdout());
            Ok(())
        }
        Command::Online(command) => {
            let output_mode = OutputMode::from_json_flag(json);
            run_online_command(router, output_mode, command).await
        }
    }
}

async fn run_online_command(
    router: String,
    output_mode: OutputMode,
    command: OnlineCommand,
) -> Result<()> {
    let app = AppContext::new(&router).await?;

    let result = match command {
        OnlineCommand::List { target } => commands::list::run(&app, output_mode, target).await,
        OnlineCommand::Watch => commands::watch::run(&app, output_mode).await,
        OnlineCommand::Graph => commands::graph::run(&app, output_mode).await,
        OnlineCommand::Schema {
            type_name,
            node,
            schema_hash,
        } => commands::schema::run(&app, output_mode, &node, &type_name, &schema_hash).await,
        OnlineCommand::Parameter { command } => {
            commands::parameter::run(&app, output_mode, command).await
        }
        OnlineCommand::Echo {
            topic,
            count,
            timeout,
        } => commands::echo::run(&app, output_mode, &topic, count, timeout).await,
        OnlineCommand::Hz(args) => {
            commands::hz::run(&app, output_mode, &args.topic, args.window, args.limit()).await
        }
        OnlineCommand::Info { target, name } => {
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
