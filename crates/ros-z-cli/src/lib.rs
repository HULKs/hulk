//! Scriptable command-line companion for `ros-z` graphs, schemas, topics, and
//! parameters.

mod app;
pub mod cli;
mod commands;
mod model;
mod render;
mod support;

use std::process::ExitCode;

use clap::CommandFactory;
use color_eyre::eyre::{Result, bail};

use crate::{
    app::AppContext,
    cli::{Cli, Command, OnlineCommand},
    render::OutputMode,
};

/// Run the CLI with parsed command-line arguments.
pub async fn run(cli: Cli) -> Result<ExitCode> {
    let Cli {
        router,
        json,
        command,
    } = cli;

    match command {
        Command::Completions { shell } => {
            let mut command = Cli::command();
            clap_complete::generate(shell, &mut command, "rosz", &mut std::io::stdout());
            Ok(ExitCode::SUCCESS)
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
) -> Result<ExitCode> {
    match command {
        OnlineCommand::Record { output, topics } => {
            if output_mode == OutputMode::Json {
                bail!("record does not support --json output in V1");
            }
            let config = commands::record::config_from_args(output, topics)?;
            let app = AppContext::new(&router).await?;
            let result = commands::record::run(&app, config).await;
            finish_online_command(app, result, ExitCode::SUCCESS)
        }
        command => {
            let app = AppContext::new(&router).await?;
            let mut exit_code = ExitCode::SUCCESS;
            let result = match command {
                OnlineCommand::List { target } => {
                    commands::list::run(&app, output_mode, target).await
                }
                OnlineCommand::Watch => commands::watch::run(&app, output_mode).await,
                OnlineCommand::Graph => commands::graph::run(&app, output_mode).await,
                OnlineCommand::Doctor { settle_timeout } => {
                    match commands::doctor::run(&app, output_mode, settle_timeout).await {
                        Ok(has_errors) => {
                            if has_errors {
                                exit_code = ExitCode::from(1);
                            }
                            Ok(())
                        }
                        Err(error) => Err(error),
                    }
                }
                OnlineCommand::Schema {
                    type_name,
                    node,
                    schema_hash,
                } => {
                    commands::schema::run(&app, output_mode, &node, &type_name, &schema_hash).await
                }
                OnlineCommand::Parameter { command } => {
                    commands::parameter::run(&app, output_mode, command).await
                }
                OnlineCommand::Echo {
                    topic,
                    count,
                    timeout,
                } => commands::echo::run(&app, output_mode, &topic, count, timeout).await,
                OnlineCommand::Hz(args) => {
                    commands::hz::run(&app, output_mode, &args.topic, args.window, args.limit())
                        .await
                }
                OnlineCommand::Info { target, name } => {
                    commands::info::run(&app, output_mode, target, &name).await
                }
                OnlineCommand::Record { .. } => {
                    unreachable!("record command is preflighted before AppContext")
                }
            };
            finish_online_command(app, result, exit_code)
        }
    }
}

fn finish_online_command(
    app: AppContext,
    result: Result<()>,
    exit_code: ExitCode,
) -> Result<ExitCode> {
    let shutdown_result = app.shutdown();

    match (result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(exit_code),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn record_rejects_json_output_before_graph_setup() {
        let error = run_online_command(
            "tcp/127.0.0.1:1".to_string(),
            OutputMode::Json,
            OnlineCommand::Record {
                output: Some(PathBuf::from("recording.mcap")),
                topics: vec!["/alpha".to_string()],
            },
        )
        .await
        .expect_err("record should reject --json before connecting to the router");

        let message = format!("{error:#}");

        assert!(message.contains("record"));
        assert!(message.contains("--json"));
    }
}
